#![cfg(feature = "tests_that_use_llms")]

use std::fs::read_to_string;
use std::path::PathBuf;

use llama_cpp_bindings::model::params::LlamaModelParams;
use serde_json::from_str;

use paddler_agent_decision::decision_text_tokenizer::DecisionTextTokenizer;
use paddler_agent_runtime::loaded_llama_model::LoadedLlamaModel;
use paddler_test_cluster_harness::fixture_path::fixture_path;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::tokenizer_reference_entry::TokenizerReferenceEntry;

pub struct KevTextTokenizationReference {
    pub model_path: PathBuf,
    pub reference_fixture: &'static str,
}

impl KevTextTokenizationReference {
    pub fn assert_matched_by_the_decision_text_tokenizer(self) {
        let text_tokenizer = DecisionTextTokenizer {
            loaded_llama_model: LoadedLlamaModel::load(
                &agent_inference_runtime_context(1),
                &self.model_path,
                &LlamaModelParams::default().with_vocab_only(true),
            )
            .expect("the decision model vocabulary must load"),
        };
        let reference: Vec<TokenizerReferenceEntry> = from_str(
            &read_to_string(fixture_path(self.reference_fixture))
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
