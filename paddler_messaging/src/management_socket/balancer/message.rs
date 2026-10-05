use crate::jsonrpc::error::Error;
use crate::jsonrpc::error_envelope::ErrorEnvelope;
use crate::jsonrpc::response_envelope::ResponseEnvelope;
use crate::rpc_message::RpcMessage;
use serde::Deserialize;
use serde::Serialize;

use super::notification::Notification;
use crate::management_socket::agent::response::Response;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    Error(ErrorEnvelope<Error>),
    Notification(Notification),
    Response(ResponseEnvelope<Response>),
}

impl RpcMessage for Message {}
