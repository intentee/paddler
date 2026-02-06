use serde::Deserialize;
use serde::Serialize;

use super::Response;
use crate::jsonrpc::Error;
use crate::jsonrpc::ErrorEnvelope;
use crate::jsonrpc::ResponseEnvelope;
use crate::rpc_message::RpcMessage;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    Error(ErrorEnvelope<Error>),
    Response(ResponseEnvelope<Response>),
}

impl RpcMessage for Message {}
