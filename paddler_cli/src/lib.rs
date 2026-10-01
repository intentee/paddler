mod cmd;
#[cfg(feature = "cuda")]
mod cuda_disclaimer_docs;

use actix_web::rt::System;
use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use command_handler::handler::Handler as _;
use command_handler::shutdown_signal::register_shutdown_signals;
use env_logger::Builder;
use env_logger::Env;
use tokio_util::sync::CancellationToken;

use crate::cmd::agent::Agent;
use crate::cmd::balancer::Balancer;
#[cfg(feature = "cuda")]
use crate::cuda_disclaimer_docs::CUDA_DISCLAIMER_DOCS;

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
    /// Generates tokens and embeddings; connects to the balancer
    Agent(Agent),
    /// Distributes incoming requests among agents
    Balancer(Box<Balancer>),
}

pub fn run() -> Result<()> {
    System::new().block_on(async {
        Builder::from_env(Env::default().default_filter_or("info")).init();

        let shutdown: CancellationToken = register_shutdown_signals()?.into();

        match Cli::parse().command {
            Commands::Agent(handler) => handler.handle(shutdown).await,
            Commands::Balancer(handler) => (*handler).handle(shutdown).await,
        }
    })
}
