use serde::Deserialize;
use serde::Serialize;

use super::Notification;
use crate::agent::jsonrpc::Response;
use crate::jsonrpc::Error;
use crate::jsonrpc::ErrorEnvelope;
use crate::jsonrpc::ResponseEnvelope;
use crate::rpc_message::RpcMessage;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    Error(ErrorEnvelope<Error>),
    Notification(Notification),
    Response(ResponseEnvelope<Response>),
}

impl RpcMessage for Message {}
