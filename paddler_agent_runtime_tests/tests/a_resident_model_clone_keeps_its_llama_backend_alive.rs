#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::model_placement::ModelPlacement;
use paddler_inference_parameters::constants::ALL_GPU_LAYERS;
use paddler_model_card_fixture::kev_0_8b::kev_0_8b;

#[test]
fn a_resident_model_clone_keeps_its_llama_backend_alive() {
    let inference_runtime_context = agent_inference_runtime_context(1);
    let loaded_llama_model = LoadedLlamaModel::load(
        &inference_runtime_context,
        LlamaModelSettings {
            model_path: kev_0_8b()
                .cached_path()
                .expect("the kev model must be in the Hugging Face cache"),
            placement: ModelPlacement::Offloaded {
                n_gpu_layers: ALL_GPU_LAYERS,
            },
        },
    )
    .expect("the decision model must load");
    let llama_backend = Arc::downgrade(&loaded_llama_model.model.llama_backend);
    let resident_model_clone = loaded_llama_model.model.clone();

    drop(loaded_llama_model);
    drop(inference_runtime_context);

    assert_eq!(llama_backend.strong_count(), 1);

    drop(resident_model_clone);

    assert_eq!(llama_backend.strong_count(), 0);
}
