#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_model::model_error::ModelError;

use paddler_agent_model_capabilities::agent_model_capabilities_error::AgentModelCapabilitiesError;
use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_agent_text_generation_tests::qwen3_with_a_patched_chat_template::Qwen3WithAPatchedChatTemplate;
use paddler_balancer_desired_state_fixture::uncompilable_chat_template::uncompilable_chat_template;

#[test]
fn model_capabilities_refuse_a_model_whose_embedded_template_does_not_compile() {
    let qwen3_with_an_uncompilable_chat_template =
        Qwen3WithAPatchedChatTemplate::with_an_embedded_chat_template(
            &uncompilable_chat_template(),
        )
        .expect("the model with a template that does not compile must be written");

    let llama_runtime = LlamaRuntime::load(
        &qwen3_with_an_uncompilable_chat_template.model_path(),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");

    assert!(matches!(
        EstablishedModelCapabilities::establish(
            &llama_runtime.loaded_llama_model,
            &llama_runtime.inference_runtime_context,
            default_model_capabilities_settings(),
        )
        .err(),
        Some(AgentModelCapabilitiesError::ChatTemplateDoesNotCompile(
            ModelError::ChatTemplatesReported { .. }
        ))
    ));
}
