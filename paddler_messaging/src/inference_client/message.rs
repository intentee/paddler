use serde::Deserialize;
use serde::Serialize;

use super::response::Response;
use crate::jsonrpc::error::Error;
use crate::jsonrpc::error_envelope::ErrorEnvelope;
use crate::jsonrpc::response_envelope::ResponseEnvelope;
use crate::rpc_message::RpcMessage;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    Error(ErrorEnvelope<Error>),
    Response(ResponseEnvelope<Response>),
}

impl RpcMessage for Message {}
