use serde::Serialize;

pub trait RpcMessage: Send + Serialize {}
