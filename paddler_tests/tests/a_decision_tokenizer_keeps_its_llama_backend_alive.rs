#![cfg(feature = "tests_that_use_llms")]

use std::sync::Arc;

use paddler_agent_decision::decision_text_tokenizer::DecisionTextTokenizer;
use paddler_agent_pointer_head::pointer_head_input_normalization::PointerHeadInputNormalization;
use paddler_agent_runtime::llama_model_settings::LlamaModelSettings;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_inference_parameters::all_gpu_layers::ALL_GPU_LAYERS;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;

#[test]
fn a_decision_tokenizer_keeps_its_llama_backend_alive() {
    let inference_runtime_context = agent_inference_runtime_context(1);
    let loaded_llama_model = LoadedLlamaModel::load(
        &inference_runtime_context,
        LlamaModelSettings {
            model_path: KevCheckpoint::Kev0_8b.target_path("model.gguf"),
            n_gpu_layers: ALL_GPU_LAYERS,
        },
    )
    .expect("the decision model must load");
    let llama_backend = Arc::downgrade(&loaded_llama_model.model.llama_backend);
    let text_tokenizer = DecisionTextTokenizer {
        input_normalization: PointerHeadInputNormalization::Unnormalized,
        model: loaded_llama_model.model.clone(),
    };

    drop(loaded_llama_model);
    drop(inference_runtime_context);

    assert_eq!(llama_backend.strong_count(), 1);

    drop(text_tokenizer);

    assert_eq!(llama_backend.strong_count(), 0);
}
