use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::error::grammar_error::GrammarError;
use llama_cpp_bindings::error::sampler_accept_error::SamplerAcceptError;
use llama_cpp_bindings::sampled_token::SampledToken;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::LlamaToken;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;
use crate::constants::GRAMMAR_REFUSING_THE_GREEDY_SELECTION_PROMPT;
use crate::kev_checkpoint::KevCheckpoint;
use crate::llama_runtime::LlamaRuntime;
use crate::prompt_sampling_positions::PromptSamplingPositions;

const GREEDY_SELECTION_AFTER_THE_PROMPT: LlamaToken = LlamaToken::new(264);
const GRAMMAR_REFUSING_EVERY_PROSE_TOKEN: &str = r#"root ::= "{{{{{{""#;
const GRAMMAR_ROOT: &str = "root";

pub struct GrammarRefusingTheGreedySelection {
    pub runtime: LlamaRuntime,
}

impl GrammarRefusingTheGreedySelection {
    /// # Errors
    ///
    /// Returns an error when the kev model cannot load, and when the greedy selection after the prompt ends the generation, which llama.cpp aborts on instead of reporting a grammar that refuses it.
    pub fn load() -> Result<Self> {
        LlamaRuntime::load(
            &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
            agent_inference_runtime_context(1),
        )
        .context("the kev model must load")
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
    /// Returns the [`GrammarError`] of a grammar that llama.cpp cannot compile.
    pub fn grammar(&self) -> Result<LlamaSampler, GrammarError> {
        LlamaSampler::grammar(
            &self.runtime.loaded_llama_model.model,
            GRAMMAR_REFUSING_EVERY_PROSE_TOKEN,
            GRAMMAR_ROOT,
        )
    }

    /// # Errors
    ///
    /// Returns the [`GrammarError`] of a lazy grammar that llama.cpp cannot compile.
    pub fn grammar_awaiting_the_greedy_selection(&self) -> Result<LlamaSampler, GrammarError> {
        LlamaSampler::grammar_lazy(
            &self.runtime.loaded_llama_model.model,
            GRAMMAR_REFUSING_EVERY_PROSE_TOKEN,
            GRAMMAR_ROOT,
            &[],
            &[GREEDY_SELECTION_AFTER_THE_PROMPT],
        )
    }

    #[must_use]
    pub fn refusal() -> SamplerAcceptError {
        SamplerAcceptError::GrammarStateCorrupted {
            message: format!(
                "Unexpected empty grammar stack after accepting piece:  a ({})",
                GREEDY_SELECTION_AFTER_THE_PROMPT.0
            ),
        }
    }
}
