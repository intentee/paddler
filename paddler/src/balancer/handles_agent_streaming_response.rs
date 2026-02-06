use anyhow::Result;
use async_trait::async_trait;

use crate::agent::jsonrpc::Request as AgentJsonRpcRequest;
use crate::balancer::manages_senders::ManagesSenders;
use crate::balancer::manages_senders_controller::ManagesSendersController;

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
