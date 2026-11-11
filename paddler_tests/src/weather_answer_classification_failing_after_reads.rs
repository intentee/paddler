use std::num::NonZeroU32;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings_sys::LLAMA_RS_FAULT_POINT_CHAT_SESSION_READ_THROWS;
use llama_cpp_fault_injection::armed_fault::ArmedFault;

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::token_classification::TokenClassification;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;

use crate::agent_inference_runtime_context::agent_inference_runtime_context;
use crate::llama_runtime::LlamaRuntime;
use crate::prompt_sampling_positions::PromptSamplingPositions;
use crate::ready_to_sample_request::ReadyToSampleRequest;
use crate::weather_chat_session::weather_chat_session;

const PROMPT: &str = "Paddler balances";

pub struct WeatherAnswerClassificationFailingAfterReads {
    pub reads_that_succeed: usize,
}

impl WeatherAnswerClassificationFailingAfterReads {
    /// # Errors
    ///
    /// Returns an error when the model is not in the Hugging Face cache or cannot load, when its context cannot be created, when the prompt cannot be decoded, when the model embeds no chat template serving the weather question, when the greedy sampler cannot initialize, and when the generation cannot be prepared.
    pub fn single_token_generation_outcome(&self) -> Result<ContinuousBatchTerminalOutcome> {
        qwen3_0_6b()
            .cached_path()
            .context("the model must be in the Hugging Face cache")
            .and_then(|model_path| {
                LlamaRuntime::load(&model_path, agent_inference_runtime_context(1))
                    .context("the model must load")
            })
            .and_then(|runtime| self.generation_outcome_of(&runtime))
    }

    fn generation_outcome_of(
        &self,
        runtime: &LlamaRuntime,
    ) -> Result<ContinuousBatchTerminalOutcome> {
        runtime
            .create_text_generation_llama_context(1)
            .context("the context must be created")
            .and_then(|mut llama_context| {
                runtime
                    .decode_prompt_for_sampling(&mut llama_context, PROMPT)
                    .and_then(|prompt_sampling_positions| {
                        self.sampled_after_the_prompt(
                            runtime,
                            &llama_context,
                            prompt_sampling_positions,
                        )
                    })
            })
    }

    fn sampled_after_the_prompt(
        &self,
        runtime: &LlamaRuntime,
        llama_context: &LlamaContext<'_>,
        prompt_sampling_positions: PromptSamplingPositions,
    ) -> Result<ContinuousBatchTerminalOutcome> {
        let model = &runtime.loaded_llama_model.model;

        ChatTemplates::new(model)
            .context("the model must embed a chat template")
            .and_then(|chat_templates| {
                let _classification_fails = ArmedFault::after_passing(
                    LLAMA_RS_FAULT_POINT_CHAT_SESSION_READ_THROWS,
                    self.reads_that_succeed,
                );

                weather_chat_session(model, &chat_templates)
                    .context("the model's template must serve the weather question")
                    .and_then(|chat_session| {
                        LlamaSampler::greedy()
                            .context("the greedy sampler must initialize")
                            .and_then(|greedy_sampler| {
                                ReadyToSampleRequest {
                                    llama_context,
                                    prompt: PROMPT,
                                    prompt_sampling_positions,
                                    runtime,
                                }
                                .advance(|prepared_request| {
                                    PreparedGenerationRequest {
                                        max_tokens: NonZeroU32::MIN,
                                        token_classification: TokenClassification::ChatSession(
                                            chat_session,
                                        ),
                                        token_sampling: TokenSampling {
                                            chain: greedy_sampler,
                                            grammar: GrammarSampling::Unconstrained,
                                        },
                                        ..prepared_request
                                    }
                                })
                            })
                    })
            })
    }
}
