#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;

#[test]
fn a_second_model_cannot_load_while_a_model_is_resident() {
    let inference_runtime_context = agent_inference_runtime_context(1);
    let llama_model_settings = || LlamaModelSettings {
        model_path: KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        n_gpu_layers: ALL_GPU_LAYERS,
    };
    let _resident_model =
        LoadedLlamaModel::load(&inference_runtime_context, llama_model_settings())
            .expect("the first model must load");

    assert_eq!(
        LoadedLlamaModel::load(&inference_runtime_context, llama_model_settings())
            .err()
            .map(|load_error| load_error.to_string()),
        Some(AgentRuntimeError::ModelWeightsAlreadyResident.to_string())
    );
}
