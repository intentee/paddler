use crate::jsonrpc::error::Error;
use crate::jsonrpc::error_envelope::ErrorEnvelope;
use crate::jsonrpc::request_envelope::RequestEnvelope;
use crate::rpc_message::RpcMessage;
use serde::Deserialize;
use serde::Serialize;

use super::notification::Notification;
use super::request::Request;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    Error(ErrorEnvelope<Error>),
    Notification(Notification),
    Request(RequestEnvelope<Request>),
}

impl RpcMessage for Message {}
