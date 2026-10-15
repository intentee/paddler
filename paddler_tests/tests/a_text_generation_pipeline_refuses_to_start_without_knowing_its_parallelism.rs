use std::io;
use std::path::PathBuf;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_text_generation::llama_cpp_media_marker::LlamaCppMediaMarker;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_tests::inference_runtime_context_with_unknown_parallelism::inference_runtime_context_with_unknown_parallelism;
use paddler_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_refuses_to_start_without_knowing_its_parallelism() {
    assert_eq!(
        TextGenerationPipelineParts {
            chat_template_source: ChatTemplateSource::EmbeddedInModel,
            inference_runtime_context: inference_runtime_context_with_unknown_parallelism(1),
            media_marker: LlamaCppMediaMarker,
            model_path: PathBuf::from("model-that-is-never-read.gguf"),
            multimodal_projection: MultimodalProjection::NotConfigured,
        }
        .spawn_refusal()
        .await,
        Some(
            TextGenerationError::AgentRuntime(AgentRuntimeError::AvailableParallelismUnknown(
                io::Error::other("the machine hides how many threads it runs in parallel")
            ))
            .to_string()
        )
    );
}
