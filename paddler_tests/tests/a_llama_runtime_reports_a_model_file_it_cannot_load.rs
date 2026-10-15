#![cfg(feature = "tests_that_use_llms")]

use std::path::Path;

use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

#[test]
fn a_llama_runtime_reports_a_model_file_it_cannot_load() {
    assert_eq!(
        LlamaRuntime::load(
            Path::new("/paddler/definitely/missing/model.gguf"),
            agent_inference_runtime_context(1),
        )
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some("unable to load the model from /paddler/definitely/missing/model.gguf".to_owned())
    );
}
