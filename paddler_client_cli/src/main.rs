mod chat_session;
mod chat_session_event;
mod cmd;
mod prompt_load_tool;
mod prompt_parse_inference_url;
mod prompt_thinking_mode;
mod stop_reason;
mod streaming_response;
mod view_chat_panels;
mod view_panel_kind;
mod view_panel_layout;
mod view_panel_navigation;
mod view_terminal_guard;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use cmd::handler::Handler as _;
use cmd::prompt::Prompt;
use paddler_bootstrap::shutdown_signal::wait_for_shutdown_signal;
use tokio_util::sync::CancellationToken;

#[derive(Parser)]
#[command(arg_required_else_help(true), version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Prompt(Prompt),
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let shutdown = CancellationToken::new();
    let signal_shutdown = shutdown.clone();

    tokio::spawn(async move {
        if let Err(error) = wait_for_shutdown_signal().await {
            log::error!("shutdown signal listener failed: {error}");
            return;
        }
        signal_shutdown.cancel();
    });

    match Cli::parse().command {
        Commands::Prompt(handler) => handler.handle(shutdown).await,
    }
}
