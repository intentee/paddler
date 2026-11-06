#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings_sys::LLAMA_RS_FAULT_POINT_CHAT_SESSION_READ_THROWS;
use llama_cpp_fault_injection::armed_fault::ArmedFault;

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::token_classification::TokenClassification;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::weather_chat_session::weather_chat_session;

use crate::decode_prompt_for_sampling::decode_prompt_for_sampling;
use crate::ready_to_sample_request::ReadyToSampleRequest;

const PROMPT: &str = "Paddler balances";

pub struct WeatherAnswerClassificationFailingAfterReads {
    pub reads_that_succeed: usize,
}

impl WeatherAnswerClassificationFailingAfterReads {
    pub fn single_token_generation_outcome(&self) -> ContinuousBatchTerminalOutcome {
        let runtime = LlamaRuntime::load(
            &qwen3_0_6b()
                .cached_path()
                .expect("the model must be in the Hugging Face cache"),
            agent_inference_runtime_context(1),
        )
        .expect("the model must load");
        let mut llama_context = runtime
            .create_text_generation_llama_context(1)
            .expect("the context must be created");
        let batch_position = decode_prompt_for_sampling(&runtime, &mut llama_context, PROMPT);
        let model = &runtime.loaded_llama_model.model;
        let chat_templates =
            ChatTemplates::new(model).expect("the model must embed a chat template");
        let _classification_fails = ArmedFault::after_passing(
            LLAMA_RS_FAULT_POINT_CHAT_SESSION_READ_THROWS,
            self.reads_that_succeed,
        );

        ReadyToSampleRequest {
            batch_position,
            llama_context: &llama_context,
            prompt: PROMPT,
            runtime: &runtime,
        }
        .advance(|prepared_request| PreparedGenerationRequest {
            max_tokens: NonZeroU32::MIN,
            token_classification: TokenClassification::ChatSession(
                weather_chat_session(model, &chat_templates)
                    .expect("the model's template must serve the weather question"),
            ),
            token_sampling: TokenSampling {
                chain: LlamaSampler::greedy().expect("the greedy sampler must initialize"),
                grammar: GrammarSampling::Unconstrained,
            },
            ..prepared_request
        })
    }
}
