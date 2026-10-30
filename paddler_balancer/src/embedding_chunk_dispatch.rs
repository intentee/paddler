use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

use crate::dispatchable_request::DispatchableRequest;
use crate::dispatched_agent::DispatchedAgent;

pub enum EmbeddingChunkDispatch {
    Buffered(DispatchableRequest<GenerateEmbeddingBatchParams>),
    Claimed {
        batch: GenerateEmbeddingBatchParams,
        dispatched_agent: DispatchedAgent,
    },
}
