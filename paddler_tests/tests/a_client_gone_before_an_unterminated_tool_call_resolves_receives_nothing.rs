#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use llama_cpp_bindings::chat_templates::ChatTemplates;
use llama_cpp_bindings::model::add_bos::AddBos;
use llama_cpp_bindings::model::parse_special_tokens::ParseSpecialTokens;
use llama_cpp_bindings::sampling::LlamaSampler;
use llama_cpp_bindings::token::LlamaToken;
use llama_cpp_bindings::token::logit_bias::LlamaLogitBias;
use serde_json::Value;
use tokio::sync::mpsc::unbounded_channel;

use paddler_agent_text_generation::continuous_batch_terminal_outcome::ContinuousBatchTerminalOutcome;
use paddler_agent_text_generation::grammar_sampling::GrammarSampling;
use paddler_agent_text_generation::prepared_generation_request::PreparedGenerationRequest;
use paddler_agent_text_generation::token_classification::TokenClassification;
use paddler_agent_text_generation::token_sampling::TokenSampling;
use paddler_agent_text_generation::tool_call_handling::ToolCallHandling;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::get_weather_tool::get_weather_tool;
use paddler_tests::llama_runtime::LlamaRuntime;
use paddler_tests::ready_to_sample_request::ReadyToSampleRequest;
use paddler_tests::tool_call_pipeline_for_tools::tool_call_pipeline_for_tools;
use paddler_tests::weather_chat_session::weather_chat_session;

const PROMPT: &str = "Paddler balances";
const TOOL_CALL_OPEN_MARKER: &str = "<tool_call>";

#[test]
fn a_client_gone_before_an_unterminated_tool_call_resolves_receives_nothing() {
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
    let prompt_sampling_positions = runtime
        .decode_prompt_for_sampling(&mut llama_context, PROMPT)
        .expect("the prompt must decode");
    let model = &runtime.loaded_llama_model.model;
    let [tool_call_open_marker]: [LlamaToken; 1] = model
        .str_to_token(
            TOOL_CALL_OPEN_MARKER,
            AddBos::Never,
            ParseSpecialTokens::Always,
        )
        .expect("the tool call marker must tokenize")
        .try_into()
        .expect("the tool call marker must be a single token");
    let chat_templates = ChatTemplates::new(model).expect("the model must embed a chat template");
    let (gone_client_tx, gone_client_rx) = unbounded_channel();

    drop(gone_client_rx);

    assert_eq!(
        ReadyToSampleRequest {
            llama_context: &llama_context,
            prompt: PROMPT,
            prompt_sampling_positions,
            runtime: &runtime,
        }
        .advance(|prepared_request| PreparedGenerationRequest {
            generated_tokens_tx: gone_client_tx,
            max_tokens: NonZeroU32::MIN,
            token_classification: TokenClassification::ChatSession(
                weather_chat_session(model, &chat_templates)
                    .expect("the model's template must serve the weather question"),
            ),
            token_sampling: TokenSampling {
                chain: LlamaSampler::chain([
                    LlamaSampler::logit_bias(
                        model.n_vocab(),
                        &[LlamaLogitBias::new(tool_call_open_marker, f32::INFINITY)],
                    )
                    .expect("the logit bias sampler must initialize"),
                    LlamaSampler::greedy().expect("the greedy sampler must initialize"),
                ])
                .expect("the sampler chain must initialize"),
                grammar: GrammarSampling::Unconstrained,
            },
            tool_call_handling: ToolCallHandling::Parsed(
                tool_call_pipeline_for_tools(&[Tool::<Value>::from(get_weather_tool()),])
                    .expect("the tools must validate")
            ),
            ..prepared_request
        })
        .expect("the request must be ready to sample"),
        ContinuousBatchTerminalOutcome::EmitNothing
    );
}
