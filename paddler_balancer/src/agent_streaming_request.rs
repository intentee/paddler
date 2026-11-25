use std::num::NonZeroU16;
use std::sync::Arc;

use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::embedding_result::EmbeddingResult;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::inference_client::response::Response as OutgoingResponse;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use paddler_messaging::request_params::decide_params::DecideParams;
use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use paddler_messaging::requested_capability::RequestedCapability;
use paddler_messaging::streamable_result::StreamableResult;

use crate::agent_response_senders::AgentResponseSenders;
use crate::dispatch_requirement::DispatchRequirement;
use crate::into_reserved_agent_request::IntoReservedAgentRequest;
use crate::response_senders::ResponseSenders;
use crate::slot_demand::SlotDemand;

pub trait AgentStreamingRequest: IntoReservedAgentRequest {
    const REQUESTED_CAPABILITY: RequestedCapability;

    type Response: StreamableResult<Final: Send + 'static, Partial: Send + 'static>
        + Into<OutgoingResponse>
        + Send
        + 'static;

    fn dispatch_requirement(&self) -> DispatchRequirement {
        DispatchRequirement {
            requested_capability: Self::REQUESTED_CAPABILITY,
            slot_demand: self.slot_demand(),
        }
    }

    fn response_senders(
        agent_response_senders: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>>;

    fn slot_demand(&self) -> SlotDemand;
}

impl AgentStreamingRequest for ContinueFromConversationHistoryParams<ValidatedParametersSchema> {
    const REQUESTED_CAPABILITY: RequestedCapability = RequestedCapability::Conversation;

    type Response = GeneratedTokenResult;

    fn response_senders(
        AgentResponseSenders {
            generated_tokens, ..
        }: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        generated_tokens
    }

    fn slot_demand(&self) -> SlotDemand {
        SlotDemand::Exactly(NonZeroU16::MIN)
    }
}

impl AgentStreamingRequest for ContinueFromRawPromptParams {
    const REQUESTED_CAPABILITY: RequestedCapability = RequestedCapability::RawTextGeneration;

    type Response = GeneratedTokenResult;

    fn response_senders(
        AgentResponseSenders {
            generated_tokens, ..
        }: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        generated_tokens
    }

    fn slot_demand(&self) -> SlotDemand {
        SlotDemand::Exactly(NonZeroU16::MIN)
    }
}

impl AgentStreamingRequest for DecideParams {
    const REQUESTED_CAPABILITY: RequestedCapability = RequestedCapability::Decisions;

    type Response = DecisionResult;

    fn response_senders(
        AgentResponseSenders { decision, .. }: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        decision
    }

    fn slot_demand(&self) -> SlotDemand {
        SlotDemand::Exactly(self.slots_needed())
    }
}

impl AgentStreamingRequest for GenerateEmbeddingBatchParams {
    const REQUESTED_CAPABILITY: RequestedCapability = RequestedCapability::Embeddings;

    type Response = EmbeddingResult;

    fn response_senders(
        AgentResponseSenders { embedding, .. }: &AgentResponseSenders,
    ) -> &Arc<ResponseSenders<Self::Response>> {
        embedding
    }

    fn slot_demand(&self) -> SlotDemand {
        SlotDemand::UpTo(u16::try_from(self.input_batch.len()).unwrap_or(u16::MAX))
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU16;
    use std::num::NonZeroU32;

    use paddler_inference_parameters::model_default_or::ModelDefaultOr;
    use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
    use paddler_messaging::conversation_history::ConversationHistory;
    use paddler_messaging::embedding_input_document::EmbeddingInputDocument;
    use paddler_messaging::embedding_normalization_method::EmbeddingNormalizationMethod;
    use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
    use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
    use paddler_messaging::request_params::decide_params::DecideParams;
    use paddler_messaging::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
    use paddler_messaging::requested_capability::RequestedCapability;
    use paddler_request_params_fixture::many_question_decision::many_question_decision;

    use super::AgentStreamingRequest as _;
    use crate::dispatch_requirement::DispatchRequirement;
    use crate::slot_demand::SlotDemand;

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
    fn a_raw_prompt_demands_exactly_one_raw_text_generation_slot() {
        assert_eq!(
            ContinueFromRawPromptParams {
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                raw_prompt: "prompt".to_owned(),
            }
            .dispatch_requirement(),
            DispatchRequirement {
                requested_capability: RequestedCapability::RawTextGeneration,
                slot_demand: SlotDemand::Exactly(NonZeroU16::MIN),
            }
        );
    }

    #[test]
    fn a_conversation_demands_exactly_one_conversation_slot() {
        assert_eq!(
            ContinueFromConversationHistoryParams {
                add_generation_prompt: true,
                conversation_history: ConversationHistory::new(Vec::new()),
                enable_thinking: ModelDefaultOr::Explicit(false),
                grammar: None,
                max_tokens: NonZeroU32::MIN,
                parallel_tool_calls: ModelDefaultOr::ModelDefault,
                parse_tool_calls: false,
                tools: Vec::new(),
            }
            .dispatch_requirement(),
            DispatchRequirement {
                requested_capability: RequestedCapability::Conversation,
                slot_demand: SlotDemand::Exactly(NonZeroU16::MIN),
            }
        );
    }

    #[test]
    fn a_decision_demands_the_decision_slots_its_questions_need() {
        assert_eq!(
            DecideParams {
                request: many_question_decision(2),
            }
            .dispatch_requirement(),
            DispatchRequirement {
                requested_capability: RequestedCapability::Decisions,
                slot_demand: SlotDemand::Exactly(DECISION_SLOTS_MINIMUM),
            }
        );
    }

    #[test]
    fn an_embedding_batch_demands_up_to_one_slot_per_document() {
        assert_eq!(
            embedding_batch(3).dispatch_requirement(),
            DispatchRequirement {
                requested_capability: RequestedCapability::Embeddings,
                slot_demand: SlotDemand::UpTo(3),
            }
        );
    }

    #[test]
    fn an_embedding_batch_longer_than_the_slot_counter_demands_every_slot_it_can_count() {
        assert_eq!(
            embedding_batch(usize::from(u16::MAX) + 1).dispatch_requirement(),
            DispatchRequirement {
                requested_capability: RequestedCapability::Embeddings,
                slot_demand: SlotDemand::UpTo(u16::MAX),
            }
        );
    }
}
