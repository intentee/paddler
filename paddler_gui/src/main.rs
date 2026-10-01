use clap::Parser;
use clap::Subcommand;
use env_logger::Builder;
use env_logger::Env;
use iced::Result as IcedResult;
use log::info;

use paddler_gui::paddler_application::paddler_application;

fn launch_gui() -> IcedResult {
    Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("paddler_gui: ready");

    paddler_application().run()
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the desktop GUI application (default if no subcommand is given)
    Launch,
}

fn main() -> IcedResult {
    match Cli::parse().command {
        Some(Commands::Launch) | None => launch_gui(),
    }
}
