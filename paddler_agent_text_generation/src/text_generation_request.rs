use tokio::sync::mpsc::UnboundedSender;

use paddler_agent_runtime::agent_request::AgentRequest;
use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_messaging::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use paddler_messaging::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;

pub enum TextGenerationRequest {
    ContinueFromConversationHistory(
        AgentRequest<
            ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
            GeneratedTokenResult,
        >,
    ),
    ContinueFromRawPrompt(AgentRequest<ContinueFromRawPromptParams, GeneratedTokenResult>),
}

impl TextGenerationRequest {
    #[must_use]
    pub const fn generated_tokens_tx(&self) -> &UnboundedSender<GeneratedTokenResult> {
        match self {
            Self::ContinueFromConversationHistory(AgentRequest { response_tx, .. })
            | Self::ContinueFromRawPrompt(AgentRequest { response_tx, .. }) => response_tx,
        }
    }
}
