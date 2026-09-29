mod app;
use crate::app::App;

fn main() -> color_eyre::Result<()> {
    App::new().run()
}
