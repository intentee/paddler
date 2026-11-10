use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::token::LlamaToken;
use serde_json::from_str;

use paddler_agent_decision::decision_text_tokenizer::DecisionTextTokenizer;
use paddler_agent_pointer_head::pointer_head_input_normalization::PointerHeadInputNormalization;
use paddler_test_cluster_harness::fixture_path::fixture_path;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;
use crate::tokenizer_reference::TokenizerReference;
use crate::tokenizer_reference_entry::TokenizerReferenceEntry;
use crate::vocabulary_only_llama_model::VocabularyOnlyLlamaModel;

pub struct KevTokenizerReferenceComparison {
    pub model_path: PathBuf,
    pub reference_fixture: &'static str,
}

impl KevTokenizerReferenceComparison {
    /// # Errors
    ///
    /// Returns an error when the reference declares an unknown input normalization, when the vocabulary of the model cannot load, and when a reference text does not tokenize.
    pub fn decision_text_tokenizer_entries(
        &self,
        TokenizerReference {
            entries,
            input_normalization,
        }: &TokenizerReference,
    ) -> Result<Vec<TokenizerReferenceEntry>> {
        PointerHeadInputNormalization::try_from(input_normalization.as_str())
            .context("the tokenizer reference must declare a known input normalization")
            .and_then(|input_normalization| {
                VocabularyOnlyLlamaModel::load(
                    &agent_inference_runtime_context(1).llama_backend,
                    &self.model_path,
                )
                .map(|vocabulary_only_llama_model| DecisionTextTokenizer {
                    input_normalization,
                    model: Arc::new(vocabulary_only_llama_model),
                })
            })
            .and_then(|text_tokenizer| {
                entries
                    .iter()
                    .map(|TokenizerReferenceEntry { text, .. }| {
                        text_tokenizer
                            .tokenize(text)
                            .context("the reference text must tokenize")
                            .map(|tokens| TokenizerReferenceEntry {
                                text: text.clone(),
                                token_ids: tokens
                                    .into_iter()
                                    .map(|LlamaToken(token_id)| token_id)
                                    .collect(),
                            })
                    })
                    .collect()
            })
    }

    /// # Errors
    ///
    /// Returns an error when the reference fixture cannot be read or parsed.
    pub fn reference(&self) -> Result<TokenizerReference> {
        read_to_string(fixture_path(self.reference_fixture))
            .context("the tokenizer reference must be readable")
            .and_then(|reference_json| {
                from_str(&reference_json).context("the tokenizer reference must parse")
            })
    }
}
