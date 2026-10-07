#![cfg(feature = "tests_that_use_llms")]

use std::fs::read_to_string;
use std::sync::Arc;

use hf_hub::Cache;
use hf_hub::Repo;
use hf_hub::RepoType;
use llama_cpp_bindings::model::params::LlamaModelParams;
use serde::Deserialize;
use serde_json::from_str;

use paddler_agent_decision::decision_text_tokenizer::DecisionTextTokenizer;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::huggingface_model_reference::HuggingFaceModelReference;
use paddler_test_cluster_harness::model_card::ModelCard;
use paddler_test_cluster_harness::model_card::qwen3_5_0_8b::qwen3_5_0_8b;

#[derive(Deserialize)]
struct TokenizerReferenceEntry {
    text: String,
    token_ids: Vec<i32>,
}

#[test]
fn decision_text_matches_the_hugging_face_tokenizer() {
    let ModelCard {
        reference:
            HuggingFaceModelReference {
                filename,
                repo_id,
                revision,
            },
    } = qwen3_5_0_8b();
    let model_path = Cache::from_env()
        .repo(Repo::with_revision(repo_id, RepoType::Model, revision))
        .get(&filename)
        .expect("the Qwen3.5 model card must be in the Hugging Face cache");
    let text_tokenizer = DecisionTextTokenizer {
        loaded_llama_model: LoadedLlamaModel::load(
            &InferenceRuntimeContext {
                agent_name: None,
                model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
                slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
            },
            &model_path,
            &LlamaModelParams::default(),
        )
        .expect("the Qwen3.5 model must load"),
    };
    let reference: Vec<TokenizerReferenceEntry> = from_str(
        &read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../fixtures/qwen3_5_tokenizer_reference.json"
        ))
        .expect("the tokenizer reference must be readable"),
    )
    .expect("the tokenizer reference must parse");

    for TokenizerReferenceEntry { text, token_ids } in reference {
        assert_eq!(
            text_tokenizer
                .tokenize(&text)
                .expect("the reference text must tokenize")
                .into_iter()
                .map(|token| token.0)
                .collect::<Vec<i32>>(),
            token_ids,
            "{text:?}"
        );
    }
}
