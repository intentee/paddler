#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;

#[test]
fn a_second_context_for_a_capability_cannot_be_created_while_one_exists() {
    let llama_runtime = LlamaRuntime::load(
        &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let _resident_context = llama_runtime
        .create_text_generation_llama_context(1)
        .expect("the first context must be created");

    assert_eq!(
        llama_runtime
            .create_text_generation_llama_context(1)
            .err()
            .map(|context_error| context_error.to_string()),
        Some(
            AgentRuntimeError::LlamaContextAlreadyResident {
                inference_capability: InferenceCapability::TextGeneration,
            }
            .to_string()
        )
    );
}
