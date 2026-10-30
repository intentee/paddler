#![cfg(feature = "tests_that_use_llms")]

use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;

use serde_json::from_str;

use paddler_agent_decision::decision_text_tokenizer::DecisionTextTokenizer;
use paddler_agent_pointer_head::pointer_head_input_normalization::PointerHeadInputNormalization;
use paddler_test_cluster_harness::fixture_path::fixture_path;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::tokenizer_reference::TokenizerReference;
use paddler_tests::tokenizer_reference_entry::TokenizerReferenceEntry;

use crate::vocabulary_only_llama_model::VocabularyOnlyLlamaModel;

pub struct KevTokenizerReferenceComparison {
    pub model_path: PathBuf,
    pub reference_fixture: &'static str,
}

impl KevTokenizerReferenceComparison {
    pub fn assert_matched_by_the_decision_text_tokenizer(self) {
        let TokenizerReference {
            entries,
            input_normalization,
        } = from_str(
            &read_to_string(fixture_path(self.reference_fixture))
                .expect("the tokenizer reference must be readable"),
        )
        .expect("the tokenizer reference must parse");
        let text_tokenizer = DecisionTextTokenizer {
            input_normalization: PointerHeadInputNormalization::try_from(
                input_normalization.as_str(),
            )
            .expect("the tokenizer reference must declare a known input normalization"),
            model: Arc::new(VocabularyOnlyLlamaModel::load(
                &agent_inference_runtime_context(1).llama_backend,
                &self.model_path,
            )),
        };

        for TokenizerReferenceEntry { text, token_ids } in entries {
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
