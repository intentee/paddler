use serde::Deserialize;
use serde::Serialize;

use paddler_types::request_params::ContinueFromConversationHistoryParams;
use paddler_types::request_params::ContinueFromRawPromptParams;
use paddler_types::request_params::GenerateEmbeddingBatchParams;
use paddler_types::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Request {
    ContinueFromConversationHistory(
        ContinueFromConversationHistoryParams<ValidatedParametersSchema>,
    ),
    ContinueFromRawPrompt(ContinueFromRawPromptParams),
    GenerateEmbeddingBatch(GenerateEmbeddingBatchParams),
    GetChatTemplateOverride,
    GetModelMetadata,
}

impl From<ContinueFromConversationHistoryParams<ValidatedParametersSchema>> for Request {
    fn from(params: ContinueFromConversationHistoryParams<ValidatedParametersSchema>) -> Self {
        Request::ContinueFromConversationHistory(params)
    }
}

impl From<ContinueFromRawPromptParams> for Request {
    fn from(params: ContinueFromRawPromptParams) -> Self {
        Request::ContinueFromRawPrompt(params)
    }
}

impl From<GenerateEmbeddingBatchParams> for Request {
    fn from(params: GenerateEmbeddingBatchParams) -> Self {
        Request::GenerateEmbeddingBatch(params)
    }
}
