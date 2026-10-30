use std::num::NonZeroU16;
use std::sync::Arc;

use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_response_senders::AgentResponseSenders;
use crate::dispatch_requirement::DispatchRequirement;
use crate::into_reserved_agent_request::IntoReservedAgentRequest;
use crate::response_senders::ResponseSenders;
use crate::slot_demand::SlotDemand;

pub trait AgentStreamingRequest: IntoReservedAgentRequest {
    const INFERENCE_CAPABILITY: InferenceCapability;

    type Response: StreamableResult<Final: Send + 'static, Partial: Send + 'static>
        + Into<OutgoingResponse>
        + Send
        + 'static;

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>>;

    fn slot_demand(&self) -> SlotDemand;

    fn dispatch_requirement(&self) -> DispatchRequirement {
        DispatchRequirement {
            inference_capability: Self::INFERENCE_CAPABILITY,
            slot_demand: self.slot_demand(),
        }
    }
}

impl AgentStreamingRequest for ContinueFromConversationHistoryParams<ValidatedParametersSchema> {
    const INFERENCE_CAPABILITY: InferenceCapability = InferenceCapability::TextGeneration;

    type Response = GeneratedTokenResult;

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        &agent_response_senders.generated_tokens
    }

    fn slot_demand(&self) -> SlotDemand {
        SlotDemand::Exactly(NonZeroU16::MIN)
    }
}

impl AgentStreamingRequest for ContinueFromRawPromptParams {
    const INFERENCE_CAPABILITY: InferenceCapability = InferenceCapability::TextGeneration;

    type Response = GeneratedTokenResult;

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        &agent_response_senders.generated_tokens
    }

    fn slot_demand(&self) -> SlotDemand {
        SlotDemand::Exactly(NonZeroU16::MIN)
    }
}

impl AgentStreamingRequest for DecideParams {
    const INFERENCE_CAPABILITY: InferenceCapability = InferenceCapability::Decisions;

    type Response = DecisionResult;

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        &agent_response_senders.decision
    }

    fn slot_demand(&self) -> SlotDemand {
        SlotDemand::Exactly(self.required_slots())
    }
}

impl AgentStreamingRequest for GenerateEmbeddingBatchParams {
    const INFERENCE_CAPABILITY: InferenceCapability = InferenceCapability::Embeddings;

    type Response = EmbeddingResult;

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        &agent_response_senders.embedding
    }

    fn slot_demand(&self) -> SlotDemand {
        SlotDemand::UpTo(u16::try_from(self.input_batch.len()).unwrap_or(u16::MAX))
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;
    use std::num::NonZeroU32;

    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::decision_question::DecisionQuestion;
    use paddler_messaging::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
    use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::inference_capability::InferenceCapability;
    use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_messaging::request_params::decide_params::DecideParams;
    use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;

    use super::AgentStreamingRequest as _;
    use crate::dispatch_requirement::DispatchRequirement;
    use crate::slot_demand::SlotDemand;

    fn decision_question(id: &str) -> DecisionQuestion {
        DecisionQuestion {
            id: id.to_owned(),
            instructions: "Pick one".to_owned(),
            options: vec!["yes".to_owned(), "no".to_owned()],
        }
    }

    fn embedding_batch(documents: usize) -> GenerateEmbeddingBatchParams {
        GenerateEmbeddingBatchParams {
            input_batch: (0..documents)
                .map(|document_index| EmbeddingInputDocument {
                    content: "content".to_owned(),
                    id: document_index.to_string(),
                })
                .collect(),
            normalization_method: EmbeddingNormalizationMethod::None,
        }
    }

    #[test]
    fn a_text_generation_request_demands_exactly_one_text_generation_slot() {
        let one_text_generation_slot = DispatchRequirement {
            inference_capability: InferenceCapability::TextGeneration,
            slot_demand: SlotDemand::Exactly(NonZeroU16::MIN),
        };

        assert_eq!(
            [
                ContinueFromRawPromptParams {
                    grammar: None,
                    max_tokens: NonZeroU32::MIN,
                    raw_prompt: "prompt".to_owned(),
                }
                .dispatch_requirement(),
                ContinueFromConversationHistoryParams {
                    add_generation_prompt: true,
                    conversation_history: ConversationHistory::new(Vec::new()),
                    enable_thinking: false,
                    grammar: None,
                    max_tokens: NonZeroU32::MIN,
                    parse_tool_calls: false,
                    tools: Vec::new(),
                }
                .dispatch_requirement(),
            ],
            [one_text_generation_slot, one_text_generation_slot]
        );
    }

    #[test]
    fn a_decision_with_leading_questions_demands_its_state_and_question_lane_slots() {
        assert_eq!(
            DecideParams {
                last_question: decision_question("last"),
                leading_questions: vec![decision_question("leading")],
                state: "state".to_owned(),
            }
            .dispatch_requirement(),
            DispatchRequirement {
                inference_capability: InferenceCapability::Decisions,
                slot_demand: SlotDemand::Exactly(DECISION_SLOTS_MINIMUM),
            }
        );
    }

    #[test]
    fn an_embedding_batch_demands_up_to_one_slot_per_document() {
        assert_eq!(
            embedding_batch(3).dispatch_requirement(),
            DispatchRequirement {
                inference_capability: InferenceCapability::Embeddings,
                slot_demand: SlotDemand::UpTo(3),
            }
        );
    }

    #[test]
    fn an_embedding_batch_longer_than_the_slot_counter_demands_every_slot_it_can_count() {
        assert_eq!(
            embedding_batch(usize::from(u16::MAX) + 1).dispatch_requirement(),
            DispatchRequirement {
                inference_capability: InferenceCapability::Embeddings,
                slot_demand: SlotDemand::UpTo(u16::MAX),
            }
        );
    }
}
