use serde::Deserialize;
use serde::Serialize;

use super::notification::Notification;
use crate::jsonrpc::response_envelope::ResponseEnvelope;
use crate::management_socket::agent::response::Response;
use crate::rpc_message::RpcMessage;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    Notification(Notification),
    Response(ResponseEnvelope<Response>),
}

impl RpcMessage for Message {}
