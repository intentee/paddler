use crate::chat_template::ChatTemplate;
use crate::embedding_result::EmbeddingResult;
use crate::generated_token_result::GeneratedTokenResult;
use crate::model_metadata::ModelMetadata;
use serde::Deserialize;
use serde::Serialize;

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
        Self::ChatTemplateOverride(chat_template)
    }
}

impl From<EmbeddingResult> for Response {
    fn from(embedding_result: EmbeddingResult) -> Self {
        Self::Embedding(embedding_result)
    }
}

impl From<GeneratedTokenResult> for Response {
    fn from(generated_token_result: GeneratedTokenResult) -> Self {
        Self::GeneratedToken(generated_token_result)
    }
}

impl From<Option<ModelMetadata>> for Response {
    fn from(model_metadata: Option<ModelMetadata>) -> Self {
        Self::ModelMetadata(model_metadata)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::mem::discriminant;

    use super::ChatTemplate;
    use super::GeneratedTokenResult;
    use super::ModelMetadata;
    use super::Response;

    fn chat_template_payload(response: &Response) -> Option<&ChatTemplate> {
        match response {
            Response::ChatTemplateOverride(chat_template) => chat_template.as_ref(),
            Response::Embedding(_) | Response::GeneratedToken(_) | Response::ModelMetadata(_) => {
                None
            }
        }
    }

    fn model_metadata_payload(response: &Response) -> Option<&ModelMetadata> {
        match response {
            Response::ModelMetadata(model_metadata) => model_metadata.as_ref(),
            Response::ChatTemplateOverride(_)
            | Response::Embedding(_)
            | Response::GeneratedToken(_) => None,
        }
    }

    #[test]
    fn converts_some_chat_template_into_chat_template_override_variant() {
        let chat_template = ChatTemplate {
            content: "{{ messages }}".to_owned(),
        };
        let response = Response::from(Some(chat_template.clone()));

        assert_eq!(
            discriminant(&response),
            discriminant(&Response::ChatTemplateOverride(None))
        );
        assert_eq!(chat_template_payload(&response), Some(&chat_template));
    }

    #[test]
    fn converts_none_chat_template_into_chat_template_override_variant() {
        let response = Response::from(Option::<ChatTemplate>::None);

        assert_eq!(
            discriminant(&response),
            discriminant(&Response::ChatTemplateOverride(None))
        );
        assert_eq!(chat_template_payload(&response), None);
    }

    #[test]
    fn generated_token_response_is_not_chat_template_override_variant() {
        let response = Response::from(GeneratedTokenResult::ContentToken("hello".to_owned()));

        assert_ne!(
            discriminant(&response),
            discriminant(&Response::ChatTemplateOverride(None))
        );
        assert_eq!(chat_template_payload(&response), None);
    }

    #[test]
    fn converts_some_model_metadata_into_model_metadata_variant() {
        let mut metadata = BTreeMap::new();
        metadata.insert("architecture".to_owned(), "llama".to_owned());
        let response = Response::from(Some(ModelMetadata {
            metadata: metadata.clone(),
        }));

        assert_eq!(
            discriminant(&response),
            discriminant(&Response::ModelMetadata(None))
        );

        let extracted_metadata = model_metadata_payload(&response)
            .expect("invariant: Some(ModelMetadata) carries a value");

        assert_eq!(extracted_metadata.metadata, metadata);
    }

    #[test]
    fn converts_none_model_metadata_into_model_metadata_variant() {
        let response = Response::from(Option::<ModelMetadata>::None);

        assert_eq!(
            discriminant(&response),
            discriminant(&Response::ModelMetadata(None))
        );
        assert!(model_metadata_payload(&response).is_none());
    }

    #[test]
    fn generated_token_response_is_not_model_metadata_variant() {
        let response = Response::from(GeneratedTokenResult::ContentToken("hello".to_owned()));

        assert_ne!(
            discriminant(&response),
            discriminant(&Response::ModelMetadata(None))
        );
        assert!(model_metadata_payload(&response).is_none());
    }
}
