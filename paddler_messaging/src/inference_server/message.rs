use serde::Deserialize;
use serde::Serialize;

use super::notification::Notification;
use super::request::Request;
use crate::jsonrpc::request_envelope::RequestEnvelope;
use crate::rpc_message::RpcMessage;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Message<TParametersSchema> {
    Notification(Notification),
    Request(RequestEnvelope<Request<TParametersSchema>>),
}

impl<TParametersSchema: Send + Serialize> RpcMessage for Message<TParametersSchema> {}
