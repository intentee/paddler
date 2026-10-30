use paddler_agent_embeddings::embedding_batch_preparer::EmbeddingBatchPreparer;
use paddler_agent_runtime::scheduler_handle::SchedulerHandle;

pub type ServedEmbeddings = SchedulerHandle<EmbeddingBatchPreparer>;
