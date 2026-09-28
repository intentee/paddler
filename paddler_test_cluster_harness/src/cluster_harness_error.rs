use std::collections::BTreeSet;
use std::net::SocketAddr;

use paddler_messaging::agent_issue::AgentIssue;
use thiserror::Error;
use tokio::time::error::Elapsed;

use crate::observation_window::ObservationWindow;
use crate::snapshots_stream::SnapshotsStream;

#[derive(Debug, Error)]
pub enum ClusterHarnessError {
    #[error("Agent {agent_id} left the balancer's pool while it was being observed")]
    AgentLeftThePool { agent_id: String },
    #[error("Agent {agent_name:?} reported issues while starting: {issues:?}")]
    AgentReportedIssues {
        agent_name: String,
        issues: BTreeSet<AgentIssue>,
    },
    #[error("Unable to build a base URL for {addr}")]
    BaseUrlInvalid {
        addr: SocketAddr,
        #[source]
        source: url::ParseError,
    },
    #[error("The balancer does not serve the OpenAI compatibility service")]
    CompatOpenAIServiceNotServed,
    #[error("The half-closed client could not reach {addr}")]
    HalfClosedClientUnreachable {
        addr: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "The {snapshots_stream:?} stream did not reach the expected state within {observation_window:?}"
    )]
    ObservationWindowElapsed {
        observation_window: ObservationWindow,
        snapshots_stream: SnapshotsStream,
        #[source]
        source: Elapsed,
    },
    #[error("The {snapshots_stream:?} stream closed before reaching the expected state")]
    SnapshotsStreamClosed { snapshots_stream: SnapshotsStream },
    #[error("The {snapshots_stream:?} stream failed")]
    SnapshotsStreamFailed {
        snapshots_stream: SnapshotsStream,
        #[source]
        source: paddler_client::error::Error,
    },
}
