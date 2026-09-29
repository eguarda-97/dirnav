use std::{fs, io, path::PathBuf};
use std::cmp::{max, min};
use crossterm::event::{self, KeyCode};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{
    Block, Borders, List, ListItem, ListState, Padding, Paragraph, ScrollbarState,
};

pub struct App {
    current_path_buf: PathBuf,
    current_list: Vec<String>,
    current_state: ListState,
    vertical: ScrollbarState,
    horizontal: ScrollbarState,
    lines: usize
}
impl App {
    pub fn new() -> Self {
        let path = fs::canonicalize("./").unwrap_or(PathBuf::default());
        App {
            current_path_buf: path.clone(),
            current_list: Vec::new(),
            current_state: ListState::default().with_selected(Some(0)),
            vertical: ScrollbarState::new(100),
            horizontal: ScrollbarState::new(100),
            lines: 0
        }
    }
    pub fn run(&mut self) -> color_eyre::Result<()> {
        color_eyre::install()?;
        ratatui::run(|terminal| {
            loop {
                terminal.draw(|frame| {
                    if let Err(e) = self.render(frame) {
                        eprintln!("Render error: {e}");
                    }
                })?;
                if !self.handle_input() {
                    break Ok(());
                };
            }
        })
    }

    fn render(&mut self, frame: &mut Frame) -> color_eyre::Result<()> {
        let outer_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(frame.area());

        let inner_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![
                Constraint::Percentage(50),
                Constraint::Percentage(50),
                Constraint::Length(1),
            ])
            .split(outer_layout[0]);

        self.render_list(frame, inner_layout[0])?;
        self.render_path_info(frame, inner_layout[1])?;
        self.render_content(frame, outer_layout[1])?;
        Ok(())
    }

    /// Render a list with directory styling.
    fn render_list(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        // 1. Retrieve the directory contents with file types
        let entries = self.list_dir()?;

        // 2. Map entries into styled ListItems
        let items: Vec<ListItem> = entries
            .into_iter()
            .map(|(file_type, name)| {
                if file_type.is_dir() {
                    // Style directories with Blue + Bold, and add a trailing slash
                    ListItem::new(format!("{name}/")).style(
                        Style::default()
                            .fg(Color::Blue)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    // Regular files default to white/reset
                    ListItem::new(name)
                }
            })
            .collect();

        // 3. Construct and render the list widget
        let list = List::new(items)
            .style(Color::White)
            .highlight_style(Modifier::REVERSED)
            .highlight_symbol("> ")
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .title(format!("On {}", self.current_path_buf.to_str().unwrap()))
                    .padding(Padding::uniform(1)),
            );

        frame.render_stateful_widget(list, area, &mut self.current_state);
        Ok(())
    }

    fn render_path_info(&self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let current_selection = PathBuf::from(self.get_selected_path());
        let current_meta = current_selection.metadata()?;
        let len = current_meta.len() as f64;
        let (size, unit) = match len {
            1e3..1e6 => (len / 1e3, "kB"),
            1e6..1e9 => (len / 1e6, "MB"),
            1e9..1e12 => (len / 1e9, "GB"),
            _ => (len, "B"),
        };
        let file_type = current_meta.file_type();
        let file_type_str = if file_type.is_dir() {
            "directory"
        } else if file_type.is_file() {
            "file"
        } else if file_type.is_symlink() {
            "symlink"
        } else {
            ""
        };
        let info = List::new(vec![
            ListItem::new(format!(
                "{}",
                current_selection.to_str().unwrap()
            )),
            ListItem::new(format!("{}", file_type_str)),
            ListItem::new(format!("{:.2} {}", size, unit)),
        ])
        .style(Color::White)
        .block(
            Block::new()
                .borders(Borders::ALL)
                .title("Path info")
                .padding(Padding::uniform(1)),
        );
        frame.render_widget(info, area);
        Ok(())
    }

    fn list_dir(&mut self) -> io::Result<Vec<(fs::FileType, String)>> {
        let mut names: Vec<(fs::FileType, String)> = fs::read_dir(&self.current_path_buf)?
            .map(|res| {
                let entry = res?;
                let name = entry.file_name().to_string_lossy().into_owned();
                Ok((entry.file_type()?, name))
            })
            .collect::<io::Result<Vec<_>>>()?;
        names.sort_by(|a, b| a.1.cmp(&b.1));

        // Update list component for other widgets
        self.current_list = names.iter().map(|val| val.clone().1).collect();
        Ok(names)
    }

    fn render_content(&mut self, frame: &mut Frame, area: Rect) -> color_eyre::Result<()> {
        let content = fs::read_to_string(self.get_selected_path()).unwrap_or(String::from(""));
        self.lines = content.lines().count();
        let p = Paragraph::new(content)
            .style(Color::White)
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .title("File content")
                    .padding(Padding::uniform(1)),
            )
            .scroll((
                self.vertical.get_position() as u16,
                self.horizontal.get_position() as u16,
            ));
        frame.render_widget(p, area);
        Ok(())
    }

    fn handle_input(&mut self) -> bool {
        if let Some(key) = event::read().unwrap().as_key_press_event() {
            match key.code {
                KeyCode::Down => {
                    self.current_state.select_next();
                    true
                }
                KeyCode::Up => {
                    self.current_state.select_previous();
                    true
                }
                KeyCode::Left => {
                    self.current_path_buf.pop();
                    self.current_state.select_first();
                    true
                }
                KeyCode::Right => {
                    if PathBuf::from(self.get_selected_path()).is_dir() {
                        self.current_path_buf.push(self.get_selected_path());
                        self.current_state.select_first();
                    }
                    true
                }
                KeyCode::Char('j') => {
                    self.vertical.next();
                    true
                }
                KeyCode::Char('k') => {
                    self.vertical.prev();
                    true
                }
                KeyCode::Char('f') => {
                    if self.lines > 30 {
                        self.vertical = self.vertical.position(min((self.lines as u16 - 30) as usize, self.vertical.get_position() + 30));
                    }
                    true
                }
                KeyCode::Char('b') => {
                    if self.lines > 30 {
                        self.vertical = self.vertical.position(max(0, self.vertical.get_position() as i16 - 30) as usize);
                    }
                    true
                }
                KeyCode::Char('q') | KeyCode::Esc => false,
                _ => true,
            }
        } else {
            true
        }
    }

    fn get_selected_path(&self) -> String {
        let mut selected_pathbuf: PathBuf = self.current_path_buf.clone();
        let idx = self.current_state.selected().unwrap_or(0);
        selected_pathbuf.push(&self.current_list[idx]);
        String::from(selected_pathbuf.to_str().unwrap_or(""))
    }
}
