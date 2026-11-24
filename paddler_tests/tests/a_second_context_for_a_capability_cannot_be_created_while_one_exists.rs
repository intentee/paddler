#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_test_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

#[test]
fn a_second_context_for_a_capability_cannot_be_created_while_one_exists() {
    let llama_runtime = LlamaRuntime::load(
        &kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the model must load");
    let _resident_context = llama_runtime
        .create_text_generation_llama_context(1)
        .expect("the first context must be created");

    assert!(matches!(
        llama_runtime.create_text_generation_llama_context(1),
        Err(AgentRuntimeError::LlamaContextAlreadyResident {
            inference_capability: InferenceCapability::TextGeneration,
        })
    ));
}
