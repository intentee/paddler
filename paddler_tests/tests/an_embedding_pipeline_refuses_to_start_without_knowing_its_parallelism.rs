use std::io;
use std::path::PathBuf;

use tokio_util::sync::CancellationToken;

use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_embeddings::embedding_pipeline::EmbeddingPipeline;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_tests::inference_runtime_context_with_unknown_parallelism::inference_runtime_context_with_unknown_parallelism;

#[tokio::test(flavor = "multi_thread")]
async fn an_embedding_pipeline_refuses_to_start_without_knowing_its_parallelism() {
    assert_eq!(
        EmbeddingPipeline {
            inference_runtime_context: inference_runtime_context_with_unknown_parallelism(1),
            model_path: PathBuf::from("model-that-is-never-read.gguf"),
            model_runtime_parameters: ModelRuntimeParameters::default(),
            pooling_type: PoolingType::Last,
        }
        .spawn(&CancellationToken::new())
        .await
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(
            EmbeddingError::AgentRuntime(AgentRuntimeError::AvailableParallelismUnknown(
                io::Error::other("the machine hides how many threads it runs in parallel")
            ))
            .to_string()
        )
    );
}
