#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;

use paddler_agent_decision::decision_text_tokenizer::DecisionTextTokenizer;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_inference_parameters::constants::ALL_GPU_LAYERS;
use paddler_test_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;

#[test]
fn a_decision_tokenizer_keeps_its_llama_backend_alive() {
    let inference_runtime_context = agent_inference_runtime_context(1);
    let loaded_llama_model = LoadedLlamaModel::load(
        &inference_runtime_context,
        LlamaModelSettings {
            model_path: kev_0_8b()
                .cached_path()
                .expect("the kev model must be in the Hugging Face cache"),
            n_gpu_layers: ALL_GPU_LAYERS,
        },
    )
    .expect("the decision model must load");
    let llama_backend = Arc::downgrade(&loaded_llama_model.model.llama_backend);
    let text_tokenizer = DecisionTextTokenizer {
        model: loaded_llama_model.model.clone(),
    };

    drop(loaded_llama_model);
    drop(inference_runtime_context);

    assert_eq!(llama_backend.strong_count(), 1);

    drop(text_tokenizer);

    assert_eq!(llama_backend.strong_count(), 0);
}
