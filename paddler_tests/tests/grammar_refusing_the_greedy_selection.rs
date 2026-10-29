#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::error::SamplerAcceptError;
use llama_cpp_bindings::sampled_token::SampledToken;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::LlamaToken;

use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::llama_runtime::LlamaRuntime;

use crate::decode_prompt_for_sampling::decode_prompt_for_sampling;

const GREEDY_SELECTION_AFTER_THE_PROMPT: LlamaToken = LlamaToken::new(264);
const GRAMMAR_REFUSING_EVERY_PROSE_TOKEN: &str = r#"root ::= "{{{{{{""#;
const GRAMMAR_ROOT: &str = "root";

pub struct GrammarRefusingTheGreedySelection {
    pub runtime: LlamaRuntime,
}

impl GrammarRefusingTheGreedySelection {
    pub const PROMPT: &'static str = "Paddler balances";

    pub fn load() -> Self {
        let runtime = LlamaRuntime::load(
            &KevCheckpoint::Kev0_8b.target_path("model.gguf"),
            agent_inference_runtime_context(1),
        )
        .expect("the kev model must load");

        assert!(
            !runtime
                .loaded_llama_model
                .model
                .is_eog_token(&SampledToken::Content(GREEDY_SELECTION_AFTER_THE_PROMPT)),
            "llama.cpp aborts instead of reporting a grammar that refuses an end-of-generation token"
        );

        Self { runtime }
    }

    pub fn refusal() -> SamplerAcceptError {
        SamplerAcceptError::GrammarStateCorrupted {
            message: format!(
                "Unexpected empty grammar stack after accepting piece:  a ({})",
                GREEDY_SELECTION_AFTER_THE_PROMPT.0
            ),
        }
    }

    pub fn decode_the_prompt(&self, llama_context: &mut LlamaContext<'_>) -> Result<i32> {
        decode_prompt_for_sampling(&self.runtime, llama_context, Self::PROMPT)
    }

    pub fn grammar(&self) -> LlamaSampler {
        LlamaSampler::grammar(
            &self.runtime.loaded_llama_model.model,
            GRAMMAR_REFUSING_EVERY_PROSE_TOKEN,
            GRAMMAR_ROOT,
        )
        .expect("the grammar must compile")
    }

    pub fn grammar_awaiting_the_greedy_selection(&self) -> LlamaSampler {
        LlamaSampler::grammar_lazy(
            &self.runtime.loaded_llama_model.model,
            GRAMMAR_REFUSING_EVERY_PROSE_TOKEN,
            GRAMMAR_ROOT,
            &[],
            &[GREEDY_SELECTION_AFTER_THE_PROMPT],
        )
        .expect("the lazy grammar must compile")
    }
}
