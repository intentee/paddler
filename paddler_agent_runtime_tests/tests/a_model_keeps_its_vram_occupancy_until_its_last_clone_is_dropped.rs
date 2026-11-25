#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_inference_parameters::constants::ALL_GPU_LAYERS;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;

#[test]
fn a_model_keeps_its_vram_occupancy_until_its_last_clone_is_dropped() {
    let inference_runtime_context = agent_inference_runtime_context(1);
    let model_weights_residency = inference_runtime_context
        .vram_residencies
        .model_weights
        .clone();
    let resident_model = LoadedLlamaModel::load(
        &inference_runtime_context,
        LlamaModelSettings {
            model_path: kev_0_8b()
                .cached_path()
                .expect("the kev model must be in the Hugging Face cache"),
            n_gpu_layers: ALL_GPU_LAYERS,
        },
    )
    .expect("the model must load");
    let resident_model_clone = resident_model.clone();

    drop(resident_model);

    assert_eq!(
        model_weights_residency
            .try_occupy()
            .err()
            .map(|occupancy_error| occupancy_error.to_string()),
        Some(AgentRuntimeError::ModelWeightsAlreadyResident.to_string())
    );

    drop(resident_model_clone);

    assert!(model_weights_residency.try_occupy().is_ok());
}
