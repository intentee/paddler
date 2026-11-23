use std::io;

use anyhow::Error as AnyhowError;
use thiserror::Error;

use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_model_source::model_source_error::ModelSourceError;

#[derive(Debug, Error)]
pub enum CliError {
    #[error("Unable to announce the balancer addresses")]
    AddressesUnannounceable(#[source] io::Error),
    #[error(transparent)]
    Bootstrap(#[from] BootstrapError),
    #[error("Unable to set up the model downloader")]
    ModelDownloaderUnconfigurable(#[source] ModelSourceError),
    #[error("Unable to register the shutdown signal handlers")]
    ShutdownSignalsUnregistrable(#[source] AnyhowError),
    #[error(transparent)]
    SocketAddrUnresolvable(AnyhowError),
}
