use paddler_agent_embeddings::embedding_batch_preparer::EmbeddingBatchPreparer;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;
use paddler_agent_runtime::shuts_down::ShutsDown;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

pub enum ServedEmbeddings {
    NotServed,
    Served(SchedulerHandle<EmbeddingBatchPreparer>),
}

impl ServedEmbeddings {
    pub fn forward(
        &self,
        agent_name: Option<&str>,
        request: AgentRequest<GenerateEmbeddingBatchParams, EmbeddingResult>,
    ) {
        match self {
            Self::NotServed => {
                EmbeddingError::report_embeddings_not_served(agent_name, &request.response_tx);
            }
            Self::Served(scheduler_handle) => scheduler_handle.request_preparer.prepare(request),
        }
    }
}

impl ShutsDown for ServedEmbeddings {
    async fn shut_down(self) {
        match self {
            Self::NotServed => {}
            Self::Served(scheduler_handle) => scheduler_handle.shut_down().await,
        }
    }
}
