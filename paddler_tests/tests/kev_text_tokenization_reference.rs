#![cfg(feature = "tests_that_use_llms")]

use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;

use llama_cpp_bindings::model::params::LlamaModelParams;
use serde_json::from_str;

use paddler_agent_decision::decision_text_tokenizer::DecisionTextTokenizer;
use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_agent_runtime::model_metadata_holder::ModelMetadataHolder;
use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_tests::tokenizer_reference_entry::TokenizerReferenceEntry;

pub struct KevTextTokenizationReference {
    pub model_path: PathBuf,
    pub reference_fixture: &'static str,
}

impl KevTextTokenizationReference {
    pub fn assert_matched_by_the_decision_text_tokenizer(self) {
        let text_tokenizer = DecisionTextTokenizer {
            loaded_llama_model: LoadedLlamaModel::load(
                &InferenceRuntimeContext {
                    agent_name: None,
                    model_metadata_holder: Arc::new(ModelMetadataHolder::default()),
                    slot_aggregated_status: Arc::new(SlotAggregatedStatus::new(1)),
                },
                &self.model_path,
                &LlamaModelParams::default().with_vocab_only(true),
            )
            .expect("the decision model vocabulary must load"),
        };
        let reference: Vec<TokenizerReferenceEntry> = from_str(
            &read_to_string(format!(
                "{}/../fixtures/{}",
                env!("CARGO_MANIFEST_DIR"),
                self.reference_fixture
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
}
