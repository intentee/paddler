use serde::Deserialize;
use serde::Serialize;

use crate::request_params::continue_from_raw_prompt_params::ContinueFromRawPromptParams;
use crate::request_params::generate_embedding_batch_params::GenerateEmbeddingBatchParams;
use crate::request_params::continue_from_conversation_history_params::ContinueFromConversationHistoryParams;
use crate::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::validated_parameters_schema::ValidatedParametersSchema;

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
        Self::ContinueFromConversationHistory(params)
    }
}

impl From<ContinueFromRawPromptParams> for Request {
    fn from(params: ContinueFromRawPromptParams) -> Self {
        Self::ContinueFromRawPrompt(params)
    }
}

impl From<GenerateEmbeddingBatchParams> for Request {
    fn from(params: GenerateEmbeddingBatchParams) -> Self {
        Self::GenerateEmbeddingBatch(params)
    }
}
