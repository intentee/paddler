use std::io;

use anyhow::Error as AnyhowError;
use hf_hub::HFError;
use reqwest::Error as ReqwestError;
use tokio::sync::watch::error::RecvError;
use tokio::task::JoinError;

#[derive(Debug, thiserror::Error)]
pub enum AgentTestsError {
    #[error("the blocking thread occupant was not released: {0}")]
    BlockingThreadOccupantUnreleased(#[source] JoinError),

    #[error("unable to create the download cache: {0}")]
    DownloadCacheUncreatable(#[source] io::Error),

    #[error("unable to start the Hugging Face client: {0}")]
    HuggingFaceClientUnstartable(#[source] HFError),

    #[error("unable to start the model downloader: {0}")]
    ModelDownloaderUnstartable(#[source] ReqwestError),

    #[error("the reconciliation service did not stop cleanly: {0}")]
    ReconciliationServiceFailed(#[source] AnyhowError),

    #[error("the reconciliation service panicked: {0}")]
    ReconciliationServicePanicked(#[source] JoinError),

    #[error("the agent never began the replaced desired state: {0}")]
    ReplacedDesiredStateNeverBegun(#[source] RecvError),

    #[error("the replacement never started downloading after the replaced worker ran: {0}")]
    ReplacementDownloadNeverStarted(#[source] RecvError),

    #[error("the stalled model server failed to serve a request: {0}")]
    StalledModelServerFailed(#[source] io::Error),

    #[error("the stalled model server panicked: {0}")]
    StalledModelServerPanicked(#[source] JoinError),

    #[error("unable to start the stalled model server: {0}")]
    StalledModelServerUnstartable(#[source] io::Error),

    #[error("unable to build the test runtime: {0}")]
    TestRuntimeUnbuildable(#[source] io::Error),
}
