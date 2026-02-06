use serde::Deserialize;
use serde::Serialize;

use super::Notification;
use super::Request;
use crate::jsonrpc::Error;
use crate::jsonrpc::ErrorEnvelope;
use crate::jsonrpc::RequestEnvelope;
use crate::rpc_message::RpcMessage;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    Error(ErrorEnvelope<Error>),
    Notification(Notification),
    Request(RequestEnvelope<Request>),
}

impl RpcMessage for Message {}
