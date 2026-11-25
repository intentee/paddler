#![cfg(feature = "tests_that_use_llms")]

use std::ffi::CString;

use llama_cpp_model::model_error::ModelError;
use tempfile::TempDir;

use paddler_agent_model_capabilities::agent_model_capabilities_error::AgentModelCapabilitiesError;
use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_pipeline_harness::write_patched_gguf::write_patched_gguf;
use paddler_agent_text_generation_tests::constants::CHAT_TEMPLATE_GGUF_KEY;
use paddler_agent_text_generation_tests::default_model_capabilities_settings::default_model_capabilities_settings;
use paddler_balancer_desired_state_fixture::constants::UNCOMPILABLE_CHAT_TEMPLATE_CONTENT;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;

#[test]
fn model_capabilities_refuse_a_model_whose_embedded_template_does_not_compile() {
    let model_directory = TempDir::new().expect("a temporary model directory must be creatable");
    let model_path = model_directory
        .path()
        .join("qwen3-with-an-embedded-template-that-does-not-compile.gguf");
    let chat_template_key =
        CString::new(CHAT_TEMPLATE_GGUF_KEY).expect("the chat template key must hold no NUL byte");
    let uncompilable_chat_template = CString::new(UNCOMPILABLE_CHAT_TEMPLATE_CONTENT)
        .expect("the uncompilable chat template must hold no NUL byte");

    write_patched_gguf(
        &qwen3_0_6b()
            .cached_path()
            .expect("Qwen3 must be in the Hugging Face cache"),
        &model_path,
        |gguf_context| {
            gguf_context.set_val_str(&chat_template_key, &uncompilable_chat_template);

            Ok(())
        },
    )
    .expect("the model with a template that does not compile must be writable");

    let llama_runtime = LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
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
