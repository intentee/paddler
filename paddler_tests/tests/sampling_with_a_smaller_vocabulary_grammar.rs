#![cfg(feature = "tests_that_use_llms")]

use std::path::Path;

use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::data_array::LlamaTokenDataArray;

use paddler_agent_text_generation::sampling_outcome::SamplingOutcome;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_agent_text_generation::token_sampling::TokenSampling;

use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::decode_prompt_for_sampling::decode_prompt_for_sampling;
use crate::vocabulary_only_llama_model::VocabularyOnlyLlamaModel;

pub struct SamplingWithASmallerVocabularyGrammar {
    pub runtime: LlamaRuntime,
    pub smaller_vocabulary_model: VocabularyOnlyLlamaModel,
}

impl SamplingWithASmallerVocabularyGrammar {
    pub fn load(smaller_vocabulary_model_path: &Path) -> Self {
        let runtime = LlamaRuntime::load(
            &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
            agent_inference_runtime_context(1),
        )
        .expect("the kev model must load");
        let smaller_vocabulary_model = VocabularyOnlyLlamaModel::load(
            &runtime.inference_runtime_context.llama_backend,
            smaller_vocabulary_model_path,
        );

        Self {
            runtime,
            smaller_vocabulary_model,
        }
    }

    pub fn sample_the_prompt_with(
        &self,
        token_sampling: impl FnOnce(LlamaSampler) -> TokenSampling,
    ) -> Result<SamplingOutcome, TextGenerationError> {
        let mut llama_context = self
            .runtime
            .create_text_generation_llama_context(1)
            .expect("the text generation context must be created");
        let batch_position =
            decode_prompt_for_sampling(&self.runtime, &mut llama_context, "Paddler balances")
                .expect("the prompt must be decoded for sampling");

        token_sampling(
            LlamaSampler::grammar(&self.smaller_vocabulary_model, r#"root ::= "a""#, "root")
                .expect("the grammar of the smaller vocabulary must initialize"),
        )
        .sample(
            &llama_context,
            batch_position,
            &mut LlamaTokenDataArray::new(Vec::new(), false),
        )
    }
}
