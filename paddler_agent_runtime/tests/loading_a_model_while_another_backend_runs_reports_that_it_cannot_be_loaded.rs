use std::mem::discriminant;
use std::path::PathBuf;

use llama_cpp_bindings::LlamaCppError;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::model::params::LlamaModelParams;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::inference_runtime_context_fixture::inference_runtime_context_fixture;

#[test]
fn loading_a_model_while_another_backend_runs_reports_that_it_cannot_be_loaded() {
    let inference_runtime_context = inference_runtime_context_fixture();
    let model_path = PathBuf::from("model-that-is-never-read.gguf");
    let _running_backend = LlamaBackend::init().expect("the first backend must initialize");

    let load_error = LoadedLlamaModel::load(
        &inference_runtime_context,
        &model_path,
        &LlamaModelParams::default(),
    )
    .err()
    .expect("a second backend must not initialize");

    assert_eq!(
        discriminant(&load_error),
        discriminant(&AgentRuntimeError::BackendInitializationFailed(
            LlamaCppError::BackendAlreadyInitialized
        ))
    );
    assert!(inference_runtime_context.slot_aggregated_status.has_issue(
        &AgentIssue::ModelCannotBeLoaded(ModelPath::from(model_path.as_path()))
    ));
}
