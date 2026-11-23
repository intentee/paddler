pub mod cli_error;
mod cmd;
pub mod constants;

use actix_web::rt::System;
use clap::Parser;
use clap::Subcommand;
use command_handler::shutdown_signal::register_shutdown_signals;
use env_logger::Builder;
use env_logger::Env;
use tokio_util::sync::CancellationToken;

use crate::cli_error::CliError;
use crate::cmd::agent::Agent;
use crate::cmd::balancer::Balancer;
use crate::cmd::singleton::Singleton;
#[cfg(feature = "cuda")]
use crate::constants::CUDA_DISCLAIMER_DOCS;

#[derive(Parser)]
#[command(arg_required_else_help(true), version, about, long_about = None)]
#[cfg_attr(feature = "cuda", command(before_help = CUDA_DISCLAIMER_DOCS))]
/// `LLMOps` platform for hosting and scaling open-source LLMs in your own infrastructure
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Runs inference for the balancer it connects to
    Agent(Agent),
    /// Distributes incoming requests among agents
    Balancer(Box<Balancer>),
    /// Runs the balancer together with its own agent in a single process
    Singleton(Box<Singleton>),
}

/// # Errors
///
/// Returns [`CliError::ShutdownSignalsUnregistrable`] when the shutdown signal handlers cannot be registered, and the [`CliError`] of the chosen command when it fails.
pub fn run() -> Result<(), CliError> {
    System::new().block_on(async {
        Builder::from_env(Env::default().default_filter_or("info")).init();

        let shutdown: CancellationToken = register_shutdown_signals()
            .map_err(CliError::ShutdownSignalsUnregistrable)?
            .into();

        match Cli::parse().command {
            Commands::Agent(agent) => agent.run(shutdown).await,
            Commands::Balancer(balancer) => (*balancer).run(shutdown).await,
            Commands::Singleton(singleton) => (*singleton).run(shutdown).await,
        }
    })
}
