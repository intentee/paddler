use serde::Deserialize;
use serde::Serialize;

use crate::chat_template::ChatTemplate;
use crate::embedding_result::EmbeddingResult;
use crate::generated_token_result::GeneratedTokenResult;
use crate::model_metadata::ModelMetadata;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum Response {
    ChatTemplateOverride(Option<ChatTemplate>),
    Embedding(EmbeddingResult),
    GeneratedToken(GeneratedTokenResult),
    ModelMetadata(Option<ModelMetadata>),
}

impl From<Option<ChatTemplate>> for Response {
    fn from(chat_template: Option<ChatTemplate>) -> Self {
        Response::ChatTemplateOverride(chat_template)
    }
}

impl From<EmbeddingResult> for Response {
    fn from(embedding_result: EmbeddingResult) -> Self {
        Response::Embedding(embedding_result)
    }
}

impl From<GeneratedTokenResult> for Response {
    fn from(generated_token_result: GeneratedTokenResult) -> Self {
        Response::GeneratedToken(generated_token_result)
    }
}

impl From<Option<ModelMetadata>> for Response {
    fn from(model_metadata: Option<ModelMetadata>) -> Self {
        Response::ModelMetadata(model_metadata)
    }
}
