use paddler_types::jsonrpc::Error;
use paddler_types::jsonrpc::ErrorEnvelope;
use paddler_types::jsonrpc::ResponseEnvelope;
use paddler_types::rpc_message::RpcMessage;
use serde::Deserialize;
use serde::Serialize;

use super::Notification;
use crate::agent::jsonrpc::Response;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    Error(ErrorEnvelope<Error>),
    Notification(Notification),
    Response(ResponseEnvelope<Response>),
}

impl RpcMessage for Message {}
