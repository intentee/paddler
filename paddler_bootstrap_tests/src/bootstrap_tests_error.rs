use std::io;

use reqwest::Error as ReqwestError;
use thiserror::Error;

use paddler_bootstrap::bootstrap_error::BootstrapError;
use paddler_test_cluster_harness::cluster_harness_error::ClusterHarnessError;

#[derive(Debug, Error)]
pub enum BootstrapTestsError {
    #[error("The dashboard does not carry its configuration")]
    DashboardAttributesUnreadable(#[source] ClusterHarnessError),
    #[error("The dashboard has no readable body")]
    DashboardBodyUnreadable(#[source] ReqwestError),
    #[error("The web admin panel did not render the dashboard")]
    DashboardNotRendered(#[source] ReqwestError),
    #[error("The web admin panel did not accept connections")]
    DashboardUnreachable(#[source] ReqwestError),
    #[error("The listener occupying an address cannot report the address it occupies")]
    OccupyingListenerAddrUnreadable(#[source] io::Error),
    #[error("Unable to bind the listener that occupies an address")]
    OccupyingListenerBindFailed(#[source] io::Error),
    #[error("A balancer runner started on an address another listener occupies")]
    RunnerStartedOnATakenAddress,
    #[error("The balancer runner did not shut down cleanly")]
    RunnerShutdownFailed(#[source] BootstrapError),
    #[error("The balancer runner did not start")]
    RunnerStartFailed(#[source] BootstrapError),
    #[error("The balancer runner does not report the web admin panel address")]
    WebAdminPanelAddrUnreported,
}
