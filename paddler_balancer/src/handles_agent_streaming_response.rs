use anyhow::Result;
use async_trait::async_trait;

use crate::manages_senders::ManagesSenders;
use crate::manages_senders_controller::ManagesSendersController;
use paddler_messaging::management_socket::agent::Request as AgentJsonRpcRequest;

#[async_trait]
pub trait HandlesAgentStreamingResponse<TParams>
where
    TParams: Into<AgentJsonRpcRequest>,
{
    type SenderCollection: ManagesSenders + Send + Sync;

    async fn handle_streaming_response(
        &self,
        request_id: String,
        params: TParams,
    ) -> Result<ManagesSendersController<Self::SenderCollection>>;
}
