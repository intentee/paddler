mod cmd;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use cmd::agent::Agent;
use cmd::balancer::Balancer;
use command_handler::handler::Handler as _;
use command_handler::shutdown_signal::register_shutdown_signals;
#[cfg(feature = "web_admin_panel")]
use esbuild_metafile::instance::initialize_instance;
use tokio_util::sync::CancellationToken;

#[cfg(feature = "web_admin_panel")]
pub const ESBUILD_META_CONTENTS: &str = include_str!("../../esbuild-meta.json");

pub const CUDA_DISCLAIMER_DOCS: &str = "
This software includes NVIDIA CUDA runtime components, subject to the NVIDIA CUDA Toolkit End User License Agreement: https://docs.nvidia.com/cuda/eula/index.html
This software contains source code provided by NVIDIA Corporation.
Paddler is not affiliated with, endorsed by, or sponsored by NVIDIA Corporation.";

#[derive(Parser)]
#[command(arg_required_else_help(true), version, about, long_about = None)]
#[cfg_attr(feature = "cuda", command(before_help = CUDA_DISCLAIMER_DOCS))]
/// `LLMOps` platform for hosting and scaling open-source LLMs in your own infrastructure
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Generates tokens and embeddings; connects to the balancer
    Agent(Agent),
    /// Distributes incoming requests among agents
    Balancer(Box<Balancer>),
}

pub fn run() -> Result<()> {
    actix_web::rt::System::new().block_on(async {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

        let shutdown: CancellationToken = register_shutdown_signals()?.into();

        match Cli::parse().command {
            Some(Commands::Agent(handler)) => handler.handle(shutdown).await,
            Some(Commands::Balancer(handler)) => {
                #[cfg(feature = "web_admin_panel")]
                initialize_instance(ESBUILD_META_CONTENTS);

                (*handler).handle(shutdown).await
            }
            None => Ok(()),
        }
    })
}
