#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_model_capabilities::agent_model_capabilities_error::AgentModelCapabilitiesError;
use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

const TEMPLATE_WITH_A_NUL_BYTE: &str = "{{ messages[0].content }}\0";

#[test]
fn model_capabilities_refuse_a_chat_template_override_containing_a_nul_byte() {
    let llama_runtime = LlamaRuntime::load(
        &qwen3_0_6b()
            .cached_path()
            .expect("the model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");

    assert!(matches!(
        EstablishedModelCapabilities::establish(
            &llama_runtime.loaded_llama_model,
            &llama_runtime.inference_runtime_context,
            ModelCapabilitiesSettings {
                chat_template_source: ChatTemplateSource::Override(ChatTemplate {
                    content: TEMPLATE_WITH_A_NUL_BYTE.to_owned(),
                }),
                ..default_model_capabilities_settings()
            },
        )
        .err(),
        Some(AgentModelCapabilitiesError::ChatTemplateContainsNul(_))
    ));
}
