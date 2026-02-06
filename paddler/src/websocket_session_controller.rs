use std::marker::PhantomData;

use actix_ws::Session;
use anyhow::Result;
use async_trait::async_trait;
use paddler_types::rpc_message::RpcMessage;

use crate::controls_session::ControlsSession;

pub struct WebSocketSessionController<TResponse>
where
    TResponse: RpcMessage + Sync,
{
    session: Session,
    _marker: PhantomData<TResponse>,
}

impl<TResponse> WebSocketSessionController<TResponse>
where
    TResponse: RpcMessage + Sync,
{
    pub fn new(session: Session) -> Self {
        WebSocketSessionController {
            session,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<TResponse> ControlsSession<TResponse> for WebSocketSessionController<TResponse>
where
    TResponse: RpcMessage + Sync + 'static,
{
    async fn send_response(&mut self, message: TResponse) -> Result<()> {
        let serialized_message = serde_json::to_string(&message)?;

        self.session.text(serialized_message).await?;

        Ok(())
    }
}
