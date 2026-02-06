use paddler_types::jsonrpc::Error;
use paddler_types::jsonrpc::ErrorEnvelope;
use paddler_types::jsonrpc::RequestEnvelope;
use paddler_types::rpc_message::RpcMessage;
use serde::Deserialize;
use serde::Serialize;

use super::Request;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    Error(ErrorEnvelope<Error>),
    Request(RequestEnvelope<Request>),
}

impl RpcMessage for Message {}
