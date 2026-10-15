#![cfg(feature = "tests_that_use_llms")]

use paddler_tests::inference_runtime_context_with_unknown_parallelism::inference_runtime_context_with_unknown_parallelism;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;
use paddler_tests::llama_runtime::LlamaRuntime;

#[test]
fn a_llama_runtime_reports_a_parallelism_it_cannot_determine() {
    assert_eq!(
        LlamaRuntime::load(
            &kev_0_8b_target_path("model.gguf"),
            inference_runtime_context_with_unknown_parallelism(1),
        )
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some("unable to determine how many threads this machine runs in parallel".to_owned())
    );
}
