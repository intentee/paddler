use paddler_messaging::request_params::continue_from_conversation_history_params::tool::Tool;
use paddler_messaging::request_params::continue_from_conversation_history_params::tool::tool_params::function_call::parameters_schema::raw_parameters_schema::RawParametersSchema;
use serde::Deserialize;

use crate::compatibility::openai_service::openai_message::OpenAIMessage;
use crate::compatibility::openai_service::stream_options::StreamOptions;

#[derive(Deserialize)]
pub struct OpenAICompletionRequestParams {
    pub max_completion_tokens: Option<i32>,
    pub messages: Vec<OpenAIMessage>,
    /// This parameter is ignored here, but is required by the `OpenAI` API.
    pub model: String,
    pub stream: Option<bool>,
    pub stream_options: Option<StreamOptions>,
    #[serde(default)]
    pub tools: Vec<Tool<RawParametersSchema>>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::OpenAICompletionRequestParams;

    #[test]
    fn deserialize_text_only_request() {
        let input = json!({
            "model": "test-model",
            "messages": [
                {"role": "user", "content": "hello"}
            ]
        });

        let params: OpenAICompletionRequestParams = serde_json::from_value(input).unwrap();

        assert_eq!(params.model, "test-model");
        assert_eq!(params.messages.len(), 1);
        assert_eq!(params.messages[0].role, "user");
        assert_eq!(params.messages[0].content.text_content(), "hello");
    }

    #[test]
    fn deserialize_request_with_stream_options_include_usage_true() {
        let input = json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true,
            "stream_options": {"include_usage": true}
        });

        let params: OpenAICompletionRequestParams = serde_json::from_value(input).unwrap();

        let stream_options = params.stream_options.unwrap();

        assert!(stream_options.include_usage);
    }

    #[test]
    fn deserialize_request_without_stream_options_defaults_to_none() {
        let input = json!({
            "model": "test-model",
            "messages": [{"role": "user", "content": "hi"}],
            "stream": true
        });

        let params: OpenAICompletionRequestParams = serde_json::from_value(input).unwrap();

        assert!(params.stream_options.is_none());
    }

    #[test]
    fn deserialize_multimodal_request_with_image() {
        let input = json!({
            "model": "vision-model",
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {"type": "text", "text": "describe this image"},
                        {"type": "image_url", "image_url": {"url": "data:image/jpeg;base64,/9j/4AAQ"}}
                    ]
                }
            ]
        });

        let params: OpenAICompletionRequestParams = serde_json::from_value(input).unwrap();

        assert_eq!(params.messages.len(), 1);
        assert_eq!(
            params.messages[0].content.text_content(),
            "describe this image"
        );

        let image_urls = params.messages[0].content.image_urls();

        assert_eq!(image_urls.len(), 1);
        assert_eq!(image_urls[0].url, "data:image/jpeg;base64,/9j/4AAQ");
    }

    #[test]
    fn deserialize_multi_turn_conversation() {
        let input = json!({
            "model": "test-model",
            "messages": [
                {"role": "system", "content": "You are a helpful assistant."},
                {"role": "user", "content": "What is 2+2?"},
                {"role": "assistant", "content": "4"},
                {"role": "user", "content": "And 3+3?"}
            ]
        });

        let params: OpenAICompletionRequestParams = serde_json::from_value(input).unwrap();

        assert_eq!(params.messages.len(), 4);
    }
}
