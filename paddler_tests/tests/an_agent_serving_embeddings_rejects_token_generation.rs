#![cfg(feature = "tests_that_use_llms")]

use std::num::NonZeroU32;

use tokio::sync::mpsc;

use paddler_agent::desired_state_reconciliation::DesiredStateReconciliation;
use paddler_agent::pipeline_request::PipelineRequest;
use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::conversation_message::ConversationMessage;
use paddler_messaging::conversation_message_content::ConversationMessageContent;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::serving_pipeline_arbiter::ServingPipelineArbiter;

const EMBEDDINGS_SERVED_REJECTION: &str =
    "agent: the agent serves Embeddings, so it cannot generate tokens";

#[tokio::test(flavor = "multi_thread")]
async fn an_agent_serving_embeddings_rejects_token_generation() {
    let serving_pipeline_arbiter = ServingPipelineArbiter::start(
        nomic_embed_text_v1_5().into_embeddings_desired_state(EmbeddingParameters::default()),
        1,
    )
    .await;

    assert_eq!(
        serving_pipeline_arbiter.desired_state_reconciliation,
        DesiredStateReconciliation::Reconciled
    );

    let (raw_prompt_response_tx, mut raw_prompt_response_rx) = mpsc::unbounded_channel();
    let (_raw_prompt_stop_tx, raw_prompt_stop_rx) = mpsc::unbounded_channel();
    let (conversation_response_tx, mut conversation_response_rx) = mpsc::unbounded_channel();
    let (_conversation_stop_tx, conversation_stop_rx) = mpsc::unbounded_channel();

    serving_pipeline_arbiter.pipeline_arbiter_state.forward(
        serving_pipeline_arbiter
            .inference_runtime_context
            .agent_name
            .as_deref(),
        PipelineRequest::ContinueFromRawPrompt(AgentRequest {
            params: ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "Hello".to_owned(),
            },
            response_tx: raw_prompt_response_tx,
            slot_guard: serving_pipeline_arbiter.slot_guard(),
            stop_rx: raw_prompt_stop_rx,
        }),
    );
    serving_pipeline_arbiter.pipeline_arbiter_state.forward(
        serving_pipeline_arbiter
            .inference_runtime_context
            .agent_name
            .as_deref(),
        PipelineRequest::ContinueFromConversationHistory(AgentRequest {
            params: ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(vec![ConversationMessage {
                    content: ConversationMessageContent::Text("Hello".to_owned()),
                    role: "user".to_owned(),
                }]),
                enable_thinking: false,
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parse_tool_calls: false,
                tools: Vec::new(),
            },
            response_tx: conversation_response_tx,
            slot_guard: serving_pipeline_arbiter.slot_guard(),
            stop_rx: conversation_stop_rx,
        }),
    );

    assert_eq!(
        raw_prompt_response_rx.recv().await,
        Some(GeneratedTokenResult::InferenceModeMismatch(
            EMBEDDINGS_SERVED_REJECTION.to_owned()
        ))
    );
    assert_eq!(
        conversation_response_rx.recv().await,
        Some(GeneratedTokenResult::InferenceModeMismatch(
            EMBEDDINGS_SERVED_REJECTION.to_owned()
        ))
    );

    serving_pipeline_arbiter
        .shut_down()
        .await
        .expect("the agent must shut down cleanly");
}
