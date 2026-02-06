use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
#[cfg(feature = "web_admin_panel")]
use esbuild_metafile::instance::initialize_instance;
use log::info;
use paddler::cmd::agent::Agent;
use paddler::cmd::balancer::Balancer;
use paddler::cmd::handler::Handler as _;
use tokio::signal::unix::SignalKind;
use tokio::signal::unix::signal;
use tokio::sync::oneshot;

#[cfg(feature = "web_admin_panel")]
pub const ESBUILD_META_CONTENTS: &str = include_str!("../../esbuild-meta.json");

#[derive(Parser)]
#[command(arg_required_else_help(true), version, about, long_about = None)]
/// LLMOps platform for hosting and scaling open-source LLMs in your own infrastructure
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[expect(clippy::large_enum_variant)]
#[derive(Subcommand)]
enum Commands {
    /// Generates tokens and embeddings; connects to the balancer
    Agent(Agent),
    /// Distributes incoming requests among agents
    Balancer(Balancer),
}

#[actix_web::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    tokio::spawn(async move {
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to listen for SIGTERM");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to listen for SIGINT");
        let mut sighup = signal(SignalKind::hangup()).expect("Failed to listen for SIGHUP");

        tokio::select! {
            _ = sigterm.recv() => info!("Received SIGTERM"),
            _ = sigint.recv() => info!("Received SIGINT (Ctrl+C)"),
            _ = sighup.recv() => info!("Received SIGHUP"),
        }

        shutdown_tx
            .send(())
            .expect("Failed to send shutdown signal");
    });

    match Cli::parse().command {
        Some(Commands::Agent(handler)) => Ok(handler.handle(shutdown_rx).await?),
        Some(Commands::Balancer(handler)) => {
            #[cfg(feature = "web_admin_panel")]
            initialize_instance(ESBUILD_META_CONTENTS);

            Ok(handler.handle(shutdown_rx).await?)
        }
        None => Ok(()),
    }
}
