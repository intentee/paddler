#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_test_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_refuses_to_start_while_its_context_is_resident() {
    let llama_runtime = LlamaRuntime::load(
        &smolvlm2_256m()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let _resident_text_generation_context = llama_runtime
        .inference_runtime_context
        .vram_residencies
        .llama_context(InferenceCapability::TextGeneration)
        .try_occupy()
        .expect("the test must occupy the text generation context residency");

    assert_eq!(
        TextGenerationPipelineParts {
            chat_template_source: ChatTemplateSource::EmbeddedInModel,
            llama_runtime,
            multimodal_projection: MultimodalProjection::NotConfigured,
        }
        .spawn_refusal()
        .await,
        Some(
            TextGenerationError::AgentRuntime(AgentRuntimeError::LlamaContextAlreadyResident {
                inference_capability: InferenceCapability::TextGeneration,
            })
            .to_string()
        )
    );
}
