pub mod address_field;
pub mod agent_running_data;
pub mod agent_running_handler;
pub mod app;
pub mod connect_address_field;
pub mod current_screen;
pub mod detect_network_interfaces;
pub mod drive_agent_stream;
pub mod drive_agent_stream_inner;
pub mod drive_balancer_loop_inner;
pub mod drive_balancer_stream;
pub mod drive_shutdown_signal_stream;
pub mod home_data;
pub mod home_handler;
pub mod join_balancer_form_data;
pub mod join_balancer_form_handler;
pub mod message;
pub mod model_preset;
pub mod network_interface_address;
pub mod running_balancer_data;
pub mod running_balancer_handler;
pub mod running_balancer_snapshot;
#[expect(unsafe_code, reason = "statum macros generate link_section statics")]
pub mod screen;
pub mod slot_count_field;
pub mod start_balancer_form_data;
pub mod start_balancer_form_handler;
pub mod started_balancer_display;
pub mod ui;

use clap::Parser;
use clap::Subcommand;
#[cfg(feature = "web_admin_panel")]
use esbuild_metafile::instance::initialize_instance;
use iced::Size;
use iced::Theme;

use crate::app::App;

#[cfg(feature = "web_admin_panel")]
const ESBUILD_META_CONTENTS: &str = include_str!("../../esbuild-meta.json");

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, PartialEq, Eq)]
pub enum Commands {
    /// Launch the desktop GUI application (default if no subcommand is given)
    Launch,
}

pub fn init_logging() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
}

pub fn run() -> iced::Result {
    init_logging();

    #[cfg(feature = "web_admin_panel")]
    initialize_instance(ESBUILD_META_CONTENTS);

    log::info!("paddler_gui: ready");

    let Cli { command } = Cli::parse();

    match command {
        Some(Commands::Launch) | None => iced::application(App::new, App::update, App::view)
            .font(include_bytes!(
                "../../resources/fonts/JetBrainsMono-Regular.ttf"
            ))
            .font(include_bytes!(
                "../../resources/fonts/JetBrainsMono-Bold.ttf"
            ))
            .theme(Theme::Light)
            .window_size(Size::new(800.0, 800.0))
            .subscription(App::subscription)
            .run(),
    }
}

#[cfg(test)]
mod tests {
    #![expect(
        clippy::unnecessary_wraps,
        reason = "tests use Result<()> uniformly so the ? operator can be added without churn"
    )]

    use anyhow::Result;
    use clap::Parser as _;

    use super::Cli;
    use super::Commands;
    use super::init_logging;

    #[test]
    fn init_logging_is_idempotent_across_repeated_invocations() -> Result<()> {
        init_logging();
        init_logging();
        Ok(())
    }

    #[test]
    fn cli_without_subcommand_parses_as_default_launch_intent() -> Result<()> {
        let cli = Cli::try_parse_from(["paddler_gui"])?;

        assert!(
            cli.command.is_none(),
            "expected no subcommand to leave Cli.command as None"
        );

        Ok(())
    }

    #[test]
    fn cli_with_launch_subcommand_parses_into_launch_variant() -> Result<()> {
        let cli = Cli::try_parse_from(["paddler_gui", "launch"])?;

        assert!(matches!(cli.command, Some(Commands::Launch)));

        Ok(())
    }

    #[test]
    fn cli_rejects_unknown_subcommands() -> Result<()> {
        let parse_result = Cli::try_parse_from(["paddler_gui", "bogus"]);

        assert!(parse_result.is_err());

        Ok(())
    }
}
