use serde::Deserialize;
use serde::Serialize;

use super::notification::Notification;
use super::request::Request;
use crate::jsonrpc::request_envelope::RequestEnvelope;
use crate::rpc_message::RpcMessage;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    Notification(Notification),
    Request(RequestEnvelope<Request>),
}

impl RpcMessage for Message {}
