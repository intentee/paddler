#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_model_capabilities::agent_model_capabilities_error::AgentModelCapabilitiesError;
use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_model_capabilities::multimodal_projection::MultimodalProjection;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;
use paddler_model_card_fixture::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;

#[test]
fn model_capabilities_refuse_a_multimodal_projection_while_another_is_resident() {
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
        EstablishedModelCapabilities::establish(
            &llama_runtime.loaded_llama_model,
            &llama_runtime.inference_runtime_context,
            ModelCapabilitiesSettings {
                multimodal_projection: MultimodalProjection::File(
                    smolvlm2_256m_mmproj()
                        .cached_path()
                        .expect("the multimodal projection must be in the Hugging Face cache"),
                ),
                ..default_model_capabilities_settings()
            },
        )
        .err(),
        Some(AgentModelCapabilitiesError::AgentRuntime(
            AgentRuntimeError::MultimodalProjectionAlreadyResident
        ))
    ));
}
