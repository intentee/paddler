use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::buffered_request_manager::BufferedRequestManager;
use crate::dispatchable_request::DispatchableRequest;
use crate::dispatched_agent::DispatchedAgent;

pub enum EmbeddingChunkDispatch {
    Buffered(DispatchableRequest<GenerateEmbeddingBatchParams>),
    Claimed {
        batch: GenerateEmbeddingBatchParams,
        dispatched_agent: DispatchedAgent,
    },
}

impl EmbeddingChunkDispatch {
    #[must_use]
    pub fn claim_or_buffer(
        dispatchable_batch: DispatchableRequest<GenerateEmbeddingBatchParams>,
        buffered_request_manager: &BufferedRequestManager,
    ) -> Self {
        match buffered_request_manager.take_available_agent(dispatchable_batch.dispatch_requirement)
        {
            Some(dispatched_agent) => Self::Claimed {
                batch: dispatchable_batch.params,
                dispatched_agent,
            },
            None => Self::Buffered(dispatchable_batch),
        }
    }
}
