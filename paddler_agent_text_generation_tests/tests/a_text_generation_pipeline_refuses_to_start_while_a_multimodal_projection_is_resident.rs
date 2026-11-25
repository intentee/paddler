#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;
use paddler_model_card_fixture::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_refuses_to_start_while_a_multimodal_projection_is_resident() {
    let llama_runtime = LlamaRuntime::load(
        &smolvlm2_256m()
            .cached_path()
            .expect("the vision model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the vision model must load");
    let _resident_projection = llama_runtime
        .inference_runtime_context
        .vram_residencies
        .multimodal_projection
        .try_occupy()
        .expect("the test must occupy the multimodal projection residency");

    assert!(matches!(
        TextGenerationPipelineParts {
            chat_template_source: ChatTemplateSource::EmbeddedInModel,
            llama_runtime,
            multimodal_projection: MultimodalProjection::File(
                smolvlm2_256m_mmproj()
                    .cached_path()
                    .expect("the multimodal projection must be in the Hugging Face cache"),
            ),
        }
        .spawn_refusal()
        .await,
        Some(TextGenerationError::AgentRuntime(
            AgentRuntimeError::MultimodalProjectionAlreadyResident
        ))
    ));
}
