use std::collections::BTreeSet;

use paddler_messaging::agent_issue::AgentIssue;
use url::Url;

#[derive(Debug, thiserror::Error)]
pub enum ClusterError {
    #[error("failed to open the {stream_path} stream")]
    StreamOpenFailed {
        stream_path: &'static str,
        #[source]
        source: anyhow::Error,
    },

    #[error("the snapshot stream yielded an error")]
    SnapshotStreamYielded {
        #[source]
        source: anyhow::Error,
    },

    #[error("the snapshot stream closed before the predicate was satisfied")]
    SnapshotStreamClosed,

    #[error(
        "agent {agent_id} disappeared from the balancer's agent pool before the predicate was satisfied"
    )]
    AgentDisappeared { agent_id: String },

    #[error("agent {agent_id} reported issues during startup: {issues:?}")]
    AgentReportedIssues {
        agent_id: String,
        issues: BTreeSet<AgentIssue>,
    },

    #[error("failed to construct the {endpoint} probe URL from {base_url}")]
    ProbeUrlConstruction {
        endpoint: String,
        base_url: Url,
        #[source]
        source: url::ParseError,
    },

    #[error("unexpected status {status} while probing {url}")]
    ProbeUnexpectedStatus { status: u16, url: Url },

    #[error("failed to probe {url}")]
    ProbeFailed {
        url: Url,
        #[source]
        source: reqwest::Error,
    },
}

pub type Result<TValue> = std::result::Result<TValue, ClusterError>;
