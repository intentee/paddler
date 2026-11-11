use std::num::NonZeroU16;
use std::num::NonZeroU32;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_embeddings::embedding_error::EmbeddingError;
use paddler_agent_runtime::rejection_description::rejection_description;
use paddler_agent_text_generation::text_generation_error::TextGenerationError;
use paddler_messaging::conversation_history::ConversationHistory;
use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::jsonrpc::request_envelope::RequestEnvelope;
use paddler_messaging::jsonrpc::response_envelope::ResponseEnvelope;
use paddler_messaging::management_socket::agent::message::Message as AgentJsonRpcMessage;
use paddler_messaging::management_socket::agent::request::Request as AgentJsonRpcRequest;
use paddler_messaging::management_socket::agent::response::Response as AgentJsonRpcResponse;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_messaging::reserved_request::ReservedRequest;
use paddler_tests::agent_connected_to_an_untrusted_balancer::AgentConnectedToAnUntrustedBalancer;

const AGENT_NAME: Option<&str> = Some("test-agent");

fn requests_of_every_kind() -> [RequestEnvelope<AgentJsonRpcRequest>; 4] {
    [
        RequestEnvelope {
            id: "conversation".to_owned(),
            request: AgentJsonRpcRequest::ContinueFromConversationHistory(ReservedRequest {
                params: ContinueFromConversationHistoryParams {
                    add_generation_prompt: true,
                    conversation_history: ConversationHistory::new(Vec::new()),
                    enable_thinking: false,
                    grammar: None,
                    max_tokens: NonZeroU32::MIN,
                    parse_tool_calls: false,
                    tools: Vec::new(),
                },
                reserved_slots: NonZeroU16::MIN,
            }),
        },
        RequestEnvelope {
            id: "decision".to_owned(),
            request: AgentJsonRpcRequest::Decide(ReservedRequest {
                params: DecideParams {
                    last_question: DecisionQuestion {
                        id: "paid".to_owned(),
                        instructions: "Was it paid?".to_owned(),
                        options: vec!["no".to_owned(), "yes".to_owned()],
                    },
                    leading_questions: Vec::new(),
                    state: "The invoice was paid on time.".to_owned(),
                },
                reserved_slots: NonZeroU16::MIN,
            }),
        },
        RequestEnvelope {
            id: "embedding-batch".to_owned(),
            request: AgentJsonRpcRequest::GenerateEmbeddingBatch(ReservedRequest {
                params: GenerateEmbeddingBatchParams {
                    input_batch: vec![EmbeddingInputDocument {
                        content: "document".to_owned(),
                        id: "document".to_owned(),
                    }],
                    normalization_method: EmbeddingNormalizationMethod::None,
                },
                reserved_slots: NonZeroU16::MIN,
            }),
        },
        RequestEnvelope {
            id: "raw-prompt".to_owned(),
            request: AgentJsonRpcRequest::ContinueFromRawPrompt(ReservedRequest {
                params: ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::MIN,
                    raw_prompt: "hello".to_owned(),
                },
                reserved_slots: NonZeroU16::MIN,
            }),
        },
    ]
}

#[tokio::test(flavor = "multi_thread")]
async fn agent_without_a_model_refuses_every_request_as_model_not_loaded() {
    let mut connected_agent = AgentConnectedToAnUntrustedBalancer::connect()
        .await
        .expect("the agent must connect to the untrusted balancer");
    let requests = requests_of_every_kind();
    let request_count = requests.len();

    for request in requests {
        connected_agent = connected_agent
            .send_agent_message(AgentJsonRpcMessage::Request(request))
            .await
            .expect("the request must be sent");
    }

    let mut responses = Vec::with_capacity(request_count);

    for _request in 0..request_count {
        responses.push(
            connected_agent
                .connection
                .next_agent_response()
                .await
                .expect("the agent must answer every request"),
        );
    }

    responses.sort_by(
        |ResponseEnvelope {
             request_id: first_request_id,
             ..
         },
         ResponseEnvelope {
             request_id: second_request_id,
             ..
         }| first_request_id.cmp(second_request_id),
    );

    assert_eq!(
        responses,
        vec![
            ResponseEnvelope {
                generated_by: None,
                request_id: "conversation".to_owned(),
                response: AgentJsonRpcResponse::GeneratedToken(
                    GeneratedTokenResult::ModelNotLoaded(rejection_description(
                        AGENT_NAME,
                        &TextGenerationError::ModelNotLoaded
                    ))
                ),
            },
            ResponseEnvelope {
                generated_by: None,
                request_id: "decision".to_owned(),
                response: AgentJsonRpcResponse::Decision(DecisionResult::ModelNotLoaded(
                    rejection_description(AGENT_NAME, &DecisionError::ModelNotLoaded)
                )),
            },
            ResponseEnvelope {
                generated_by: None,
                request_id: "embedding-batch".to_owned(),
                response: AgentJsonRpcResponse::Embedding(EmbeddingResult::ModelNotLoaded(
                    rejection_description(AGENT_NAME, &EmbeddingError::ModelNotLoaded)
                )),
            },
            ResponseEnvelope {
                generated_by: None,
                request_id: "raw-prompt".to_owned(),
                response: AgentJsonRpcResponse::GeneratedToken(
                    GeneratedTokenResult::ModelNotLoaded(rejection_description(
                        AGENT_NAME,
                        &TextGenerationError::ModelNotLoaded
                    ))
                ),
            },
        ]
    );
}
