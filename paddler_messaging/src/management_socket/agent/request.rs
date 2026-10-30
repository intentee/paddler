use serde::Deserialize;
use serde::Serialize;

use crate::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;
use crate::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use crate::request_params::decide_params::DecideParams;
use crate::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use crate::reserved_request::ReservedRequest;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Request {
    ContinueFromConversationHistory(
        ReservedRequest<ContinueFromConversationHistoryParams<ValidatedParametersSchema>>,
    ),
    ContinueFromRawPrompt(ReservedRequest<ContinueFromRawPromptParams>),
    Decide(ReservedRequest<DecideParams>),
    GenerateEmbeddingBatch(ReservedRequest<GenerateEmbeddingBatchParams>),
    GetChatTemplateOverride,
    GetModelMetadata,
}

impl From<ReservedRequest<ContinueFromConversationHistoryParams<ValidatedParametersSchema>>>
    for Request
{
    fn from(
        reserved_request: ReservedRequest<
            ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
        >,
    ) -> Self {
        Self::ContinueFromConversationHistory(reserved_request)
    }
}

impl From<ReservedRequest<ContinueFromRawPromptParams>> for Request {
    fn from(reserved_request: ReservedRequest<ContinueFromRawPromptParams>) -> Self {
        Self::ContinueFromRawPrompt(reserved_request)
    }
}

impl From<ReservedRequest<DecideParams>> for Request {
    fn from(reserved_request: ReservedRequest<DecideParams>) -> Self {
        Self::Decide(reserved_request)
    }
}

impl From<ReservedRequest<GenerateEmbeddingBatchParams>> for Request {
    fn from(reserved_request: ReservedRequest<GenerateEmbeddingBatchParams>) -> Self {
        Self::GenerateEmbeddingBatch(reserved_request)
    }
}
