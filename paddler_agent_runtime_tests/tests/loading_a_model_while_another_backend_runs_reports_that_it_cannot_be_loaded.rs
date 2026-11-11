use std::mem::discriminant;
use std::path::PathBuf;

use llama_cpp_bindings::error::llama_cpp_error::LlamaCppError;
use llama_cpp_bindings::llama_backend::LlamaBackend;

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;

#[test]
fn loading_a_model_while_another_backend_runs_reports_that_it_cannot_be_loaded() {
    let inference_runtime_context = agent_inference_runtime_context(1);
    let model_path = PathBuf::from("model-that-is-never-read.gguf");
    let _running_backend = LlamaBackend::init().expect("the first backend must initialize");

    let load_error = LoadedLlamaModel::load(
        &inference_runtime_context,
        LlamaModelSettings {
            model_path: model_path.clone(),
            n_gpu_layers: 0,
        },
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
