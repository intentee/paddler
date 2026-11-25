use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_context::llama_context::LlamaContext;
use llama_cpp_model::llama_token::LlamaToken;
use llama_cpp_model::sampled_token::SampledToken;
use llama_cpp_sampler::llama_sampler::LlamaSampler;
use llama_cpp_sampler::sampler_error::SamplerError;

use paddler_agent_pipeline_harness::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_pipeline_harness::prompt_sampling_positions::PromptSamplingPositions;
use paddler_model_card_fixture::qwen3_5_0_8b::qwen3_5_0_8b;

use crate::constants::GRAMMAR_REFUSING_THE_GREEDY_SELECTION_PROMPT;

const GREEDY_SELECTION_AFTER_THE_PROMPT: LlamaToken = LlamaToken::new(383);
const GRAMMAR_REFUSING_EVERY_PROSE_TOKEN: &str = r#"root ::= "{{{{{{""#;
const GRAMMAR_ROOT: &str = "root";

pub struct GrammarRefusingTheGreedySelection {
    pub runtime: LlamaRuntime,
}

impl GrammarRefusingTheGreedySelection {
    /// # Errors
    ///
    /// Returns an error when the Qwen3.5 model cannot load, and when the greedy selection after the prompt ends the generation, which llama.cpp aborts on instead of reporting a grammar that refuses it.
    pub fn load() -> Result<Self> {
        qwen3_5_0_8b()
            .cached_path()
            .context("the Qwen3.5 model must be in the Hugging Face cache")
            .and_then(|model_path| {
                LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
                    .context("the Qwen3.5 model must load")
            })
        .and_then(|runtime| {
            let greedy_selection_ends_the_generation = runtime
                .loaded_llama_model
                .model
                .is_eog_token(&SampledToken::Content(GREEDY_SELECTION_AFTER_THE_PROMPT));

            (!greedy_selection_ends_the_generation)
                .then_some(Self { runtime })
                .context("llama.cpp aborts instead of reporting a grammar that refuses an end-of-generation token")
        })
    }

    /// # Errors
    ///
    /// Returns the error of [`LlamaRuntime::decode_prompt_for_sampling`] when the prompt cannot be decoded.
    pub fn decode_the_prompt(
        &self,
        llama_context: &mut LlamaContext<'_>,
    ) -> Result<PromptSamplingPositions> {
        self.runtime
            .decode_prompt_for_sampling(llama_context, GRAMMAR_REFUSING_THE_GREEDY_SELECTION_PROMPT)
    }

    /// # Errors
    ///
    /// Returns the [`SamplerError`] of a grammar that llama.cpp cannot compile.
    pub fn grammar(&self) -> Result<LlamaSampler, SamplerError> {
        LlamaSampler::grammar(
            &self.runtime.loaded_llama_model.model,
            GRAMMAR_REFUSING_EVERY_PROSE_TOKEN,
            GRAMMAR_ROOT,
        )
    }

    /// # Errors
    ///
    /// Returns the [`SamplerError`] of a lazy grammar that llama.cpp cannot compile.
    pub fn grammar_awaiting_the_greedy_selection(&self) -> Result<LlamaSampler, SamplerError> {
        LlamaSampler::grammar_lazy(
            &self.runtime.loaded_llama_model.model,
            GRAMMAR_REFUSING_EVERY_PROSE_TOKEN,
            GRAMMAR_ROOT,
            &[],
            &[GREEDY_SELECTION_AFTER_THE_PROMPT],
        )
    }
}
