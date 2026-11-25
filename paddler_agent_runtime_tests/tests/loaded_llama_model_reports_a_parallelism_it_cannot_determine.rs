#![cfg(feature = "tests_that_use_llms")]

use std::io::ErrorKind;

use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime_tests::inference_runtime_context_with_unknown_parallelism::inference_runtime_context_with_unknown_parallelism;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;

#[test]
fn loaded_llama_model_reports_a_parallelism_it_cannot_determine() {
    assert!(matches!(
        LlamaRuntime::load(
            &kev_0_8b().cached_path().expect("the kev model must be in the Hugging Face cache"),
            inference_runtime_context_with_unknown_parallelism(1),
        ),
        Err(AgentRuntimeError::AvailableParallelismUnknown(parallelism_error))
            if parallelism_error.kind() == ErrorKind::Other
    ));
}
