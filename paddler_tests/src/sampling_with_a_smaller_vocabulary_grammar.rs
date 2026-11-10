use std::path::Path;

use anyhow::Context as _;
use anyhow::Error;
use anyhow::Result;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

use paddler_agent_text_generation::sampling_outcome::SamplingOutcome;
use paddler_agent_text_generation::token_sampling::TokenSampling;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;
use crate::kev_checkpoint::KevCheckpoint;
use crate::llama_runtime::LlamaRuntime;
use crate::prompt_sampling_positions::PromptSamplingPositions;
use crate::vocabulary_only_llama_model::VocabularyOnlyLlamaModel;

pub struct SamplingWithASmallerVocabularyGrammar {
    pub runtime: LlamaRuntime,
    pub smaller_vocabulary_model: VocabularyOnlyLlamaModel,
}

impl SamplingWithASmallerVocabularyGrammar {
    /// # Errors
    ///
    /// Returns an error when the kev model or the vocabulary of the smaller model cannot load.
    pub fn load(smaller_vocabulary_model_path: &Path) -> Result<Self> {
        LlamaRuntime::load(
            &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
            agent_inference_runtime_context(1),
        )
        .context("the kev model must load")
        .and_then(|runtime| {
            VocabularyOnlyLlamaModel::load(
                &runtime.inference_runtime_context.llama_backend,
                smaller_vocabulary_model_path,
            )
            .map(|smaller_vocabulary_model| Self {
                runtime,
                smaller_vocabulary_model,
            })
        })
    }

    /// # Errors
    ///
    /// Returns an error when the text generation context cannot be created, when the prompt cannot be decoded, when the grammar of the smaller vocabulary cannot initialize, and the [`TextGenerationError`](paddler_agent_text_generation::text_generation_error::TextGenerationError) of a sampling that fails.
    pub fn sample_the_prompt_with(
        &self,
        token_sampling: impl FnOnce(LlamaSampler) -> TokenSampling,
    ) -> Result<SamplingOutcome> {
        self.runtime
            .create_text_generation_llama_context(1)
            .context("the text generation context must be created")
            .and_then(|mut llama_context| {
                self.runtime
                    .decode_prompt_for_sampling(&mut llama_context, "Paddler balances")
                    .and_then(|PromptSamplingPositions { batch_position, .. }| {
                        LlamaSampler::grammar(
                            &self.smaller_vocabulary_model,
                            r#"root ::= "a""#,
                            "root",
                        )
                        .context("the grammar of the smaller vocabulary must initialize")
                        .and_then(|grammar_of_the_smaller_vocabulary| {
                            token_sampling(grammar_of_the_smaller_vocabulary)
                                .sample(
                                    &llama_context,
                                    batch_position,
                                    &mut LlamaTokenDataArray::new(Vec::new(), false),
                                )
                                .map_err(Error::from)
                        })
                    })
            })
    }
}
