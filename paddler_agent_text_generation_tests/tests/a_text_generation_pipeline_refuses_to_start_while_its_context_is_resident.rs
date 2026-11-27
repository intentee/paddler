#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_agent_text_generation_tests::text_generation_pipeline_parts::TextGenerationPipelineParts;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

#[tokio::test(flavor = "multi_thread")]
async fn a_text_generation_pipeline_refuses_to_start_while_its_context_is_resident() {
    let llama_runtime = LlamaRuntime::load(
        &smolvlm2_256m()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let model_capabilities = llama_runtime
        .establish_model_capabilities(default_model_capabilities_settings())
        .expect("the capabilities of the model must be established");
    let _resident_text_generation_context = llama_runtime
        .inference_runtime_context
        .vram_residencies
        .llama_context(InferenceCapability::TextGeneration)
        .try_occupy()
        .expect("the test must occupy the text generation context residency");

    assert!(matches!(
        TextGenerationPipelineParts {
            llama_runtime,
            model_capabilities,
        }
        .spawn_refusal()
        .await,
        Some(TextGenerationError::AgentRuntime(
            AgentRuntimeError::LlamaContextAlreadyResident {
                inference_capability: InferenceCapability::TextGeneration,
            }
        ))
    ));
}
