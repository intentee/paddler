use std::collections::BTreeSet;
use std::io::Error as IoError;
use std::net::SocketAddr;
use std::str::Utf8Error;

use async_openai::error::OpenAIError;
use quick_xml::Error as XmlError;
use quick_xml::events::attributes::AttrError;
use serde_json::Error as SerdeJsonError;
use thiserror::Error;
use tokio_tungstenite::tungstenite::Error as WebSocketError;
use url::ParseError;

use paddler_client::error::Error as ClientError;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::message::Message as InferenceMessage;
use paddler_messaging::jsonrpc::error::Error as JsonRpcError;

use crate::snapshots_stream::SnapshotsStream;

#[derive(Debug, Error)]
pub enum ClusterHarnessError {
    #[error("The embedding stream carried a message that is not an embedding result: {message:?}")]
    EmbeddingStreamMessageUnexpected { message: Box<InferenceMessage> },
    #[error("The token generation stream returned JSON-RPC error code {} ({})", error.code, error.description)]
    TokenStreamReturnedError { error: JsonRpcError },
    #[error(
        "The token generation stream carried a message that is not a generated token: {message:?}"
    )]
    TokenStreamMessageUnexpected { message: Box<InferenceMessage> },
    #[error("Agent {agent_id} left the balancer's pool while it was being observed")]
    AgentLeftThePool { agent_id: String },
    #[error("The raw agent socket could not serialize a notification")]
    AgentSocketNotificationUnserializable(#[source] SerdeJsonError),
    #[error("The raw agent socket ended without a close frame")]
    AgentSocketEndedWithoutClosing,
    #[error("The raw agent socket ended before the balancer sent a message")]
    AgentSocketEndedWithoutMessage,
    #[error("The raw agent socket could not deserialize a balancer message")]
    AgentSocketMessageUndeserializable(#[source] SerdeJsonError),
    #[error("The raw agent socket could not receive a frame")]
    AgentSocketReceiveFailed(#[source] WebSocketError),
    #[error("The raw agent socket could not serialize a response")]
    AgentSocketResponseUnserializable(#[source] SerdeJsonError),
    #[error("The raw agent socket could not send a frame")]
    AgentSocketSendFailed(#[source] WebSocketError),
    #[error("Agent {agent_name:?} reported issues while starting: {issues:?}")]
    AgentReportedIssues {
        agent_name: String,
        issues: BTreeSet<AgentIssue>,
    },
    #[error("Unable to build a base URL for {addr}")]
    BaseUrlInvalid {
        addr: SocketAddr,
        #[source]
        source: ParseError,
    },
    #[error("The balancer does not serve the OpenAI compatibility service")]
    CompatOpenAIServiceNotServed,
    #[error("The web admin panel page carries a malformed attribute")]
    DashboardAttributeMalformed(#[source] AttrError),
    #[error("The web admin panel page carries an attribute whose name is not UTF-8")]
    DashboardAttributeNameNotUtf8(#[source] Utf8Error),
    #[error("The web admin panel page carries an attribute value that cannot be unescaped")]
    DashboardAttributeValueUnreadable(#[source] XmlError),
    #[error("The web admin panel page has no dashboard element")]
    DashboardElementMissing,
    #[error("The web admin panel page cannot be read as markup")]
    DashboardMarkupUnreadable(#[source] XmlError),
    #[error(
        "The test requested another HTTP connection while holding {held_connections}; every supported system's listen backlog queues at most {limit}"
    )]
    ConcurrentConnectionsExceedPortableListenBacklog {
        held_connections: usize,
        limit: usize,
    },
    #[error("The generation did not end with a summary; its last result was {last_token_result:?}")]
    GenerationEndedWithoutSummary {
        last_token_result: Option<GeneratedTokenResult>,
    },
    #[error("The half-closed client could not reach {addr}")]
    HalfClosedClientUnreachable {
        addr: SocketAddr,
        #[source]
        source: IoError,
    },
    #[error("The inference request failed")]
    InferenceRequestFailed(#[source] ClientError),
    #[error("The raw inference socket closed before answering request {request_id}")]
    InferenceSocketClosedBeforeAnswer { request_id: String },
    #[error("The raw inference socket closed before answering any request")]
    InferenceSocketClosedBeforeAnyAnswer,
    #[error("The raw inference socket closed before answering the ping")]
    InferenceSocketClosedBeforePong,
    #[error("The raw inference socket received a message it could not read")]
    InferenceSocketMessageUnreadable(#[source] SerdeJsonError),
    #[error("The raw inference socket could not receive a frame")]
    InferenceSocketReceiveFailed(#[source] WebSocketError),
    #[error("The raw inference socket could not serialize a request")]
    InferenceSocketRequestUnserializable(#[source] SerdeJsonError),
    #[error("The raw inference socket could not send a frame")]
    InferenceSocketSendFailed(#[source] WebSocketError),
    #[error("The OpenAI compatibility request failed")]
    OpenAIRequestFailed(#[source] OpenAIError),
    #[error("Unable to list the open file descriptors of the current process")]
    OpenFileDescriptorsUnreadable(#[source] IoError),
    #[error("The {snapshots_stream:?} stream closed before reaching the expected state")]
    SnapshotsStreamClosed { snapshots_stream: SnapshotsStream },
    #[error("The {snapshots_stream:?} stream failed")]
    SnapshotsStreamFailed {
        snapshots_stream: SnapshotsStream,
        #[source]
        source: ClientError,
    },
}
