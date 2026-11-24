#![cfg(feature = "tests_that_use_llms")]

use std::path::Path;

use llama_cpp_model::model_error::ModelError;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

#[test]
fn a_llama_runtime_reports_a_model_file_it_cannot_load() {
    let missing_model_path = Path::new("/paddler/definitely/missing/model.gguf");

    assert!(matches!(
        LlamaRuntime::load(missing_model_path, agent_inference_runtime_context(1)),
        Err(AgentRuntimeError::ModelLoadFailed {
            model_path,
            source: ModelError::ModelFileNotFound(not_found_model_path),
        }) if model_path == missing_model_path && not_found_model_path == missing_model_path
    ));
}
