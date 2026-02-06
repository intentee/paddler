use serde::Deserialize;
use serde::Serialize;

use super::Request;
use crate::jsonrpc::Error;
use crate::jsonrpc::ErrorEnvelope;
use crate::jsonrpc::RequestEnvelope;
use crate::rpc_message::RpcMessage;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message<TParametersSchema: Default> {
    Error(ErrorEnvelope<Error>),
    Request(RequestEnvelope<Request<TParametersSchema>>),
}

impl<TParametersSchema: Default + Send + Serialize> RpcMessage for Message<TParametersSchema> {}
