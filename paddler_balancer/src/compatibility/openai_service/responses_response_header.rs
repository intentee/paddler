use llama_cpp_bindings_types::TokenUsage;

use paddler_messaging::generation_finish::GenerationFinish;

use crate::compatibility::openai_service::openai_error::OpenAIError;
use crate::compatibility::openai_service::responses_output_item::ResponsesOutputItem;
use crate::compatibility::openai_service::responses_response::ResponsesResponse;
use crate::compatibility::openai_service::responses_response_progress::ResponsesResponseProgress;
use crate::compatibility::openai_service::responses_usage::ResponsesUsage;

#[derive(Clone, Debug)]
pub struct ResponsesResponseHeader {
    pub created_at: u64,
    pub id: String,
    pub instructions: Option<String>,
    pub model: String,
    pub temperature: f32,
    pub top_p: f32,
}

impl ResponsesResponseHeader {
    #[must_use]
    pub fn in_progress(&self) -> ResponsesResponse {
        self.with_progress(ResponsesResponseProgress::InProgress)
    }

    #[must_use]
    pub fn finished(
        &self,
        output: Vec<ResponsesOutputItem>,
        usage: &TokenUsage,
        finish: GenerationFinish,
    ) -> ResponsesResponse {
        let usage = ResponsesUsage(*usage);

        self.with_progress(if finish.reached_a_length_limit() {
            ResponsesResponseProgress::Incomplete { output, usage }
        } else {
            ResponsesResponseProgress::Completed { output, usage }
        })
    }

    #[must_use]
    pub fn failed(&self, error: &OpenAIError) -> ResponsesResponse {
        self.with_progress(ResponsesResponseProgress::Failed {
            error_message: error.message.clone(),
        })
    }

    fn with_progress(&self, progress: ResponsesResponseProgress) -> ResponsesResponse {
        ResponsesResponse {
            header: self.clone(),
            progress,
        }
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::TokenUsage;
    use serde_json::json;
    use serde_json::to_value;

    use paddler_messaging::generation_finish::GenerationFinish;

    use super::ResponsesResponseHeader;
    use crate::compatibility::openai_service::openai_error::OpenAIError;
    use crate::compatibility::openai_service::openai_error_type::OpenAIErrorType;
    use crate::compatibility::openai_service::responses_output_item::ResponsesOutputItem;

    fn header() -> ResponsesResponseHeader {
        ResponsesResponseHeader {
            id: "resp_test".to_owned(),
            created_at: 1_234,
            model: "test-model".to_owned(),
            instructions: None,
            temperature: 0.25,
            top_p: 0.5,
        }
    }

    #[test]
    fn in_progress_reports_in_progress_status_with_no_output() {
        let response = to_value(header().in_progress()).unwrap();

        assert_eq!(response["status"], "in_progress");
        assert_eq!(response["object"], "response");
        assert_eq!(response["output"], json!([]));
        assert!(response.get("usage").is_none());
    }

    #[test]
    fn reports_the_sampling_parameters_in_effect() {
        let response = to_value(header().in_progress()).unwrap();

        assert_eq!(response["temperature"], 0.25);
        assert_eq!(response["top_p"], 0.5);
    }

    #[test]
    fn failed_carries_the_error_message() {
        let response = to_value(header().failed(&OpenAIError {
            error_type: OpenAIErrorType::ServerError,
            message: "boom".to_owned(),
        }))
        .unwrap();

        assert_eq!(response["status"], "failed");
        assert_eq!(response["error"]["message"], "boom");
        assert!(response.get("usage").is_none());
    }

    #[test]
    fn completed_includes_usage_and_output() {
        let response = to_value(header().finished(
            vec![ResponsesOutputItem::completed_message(
                "msg_0".to_owned(),
                "hello".to_owned(),
            )],
            &TokenUsage {
                prompt_tokens: 3,
                cached_prompt_tokens: 0,
                input_image_tokens: 0,
                input_audio_tokens: 0,
                content_tokens: 5,
                reasoning_tokens: 2,
                tool_call_tokens: 0,
                undeterminable_tokens: 0,
            },
            GenerationFinish::EndOfGeneration,
        ))
        .unwrap();

        assert_eq!(response["status"], "completed");
        assert_eq!(response["usage"]["input_tokens"], 3);
        assert_eq!(response["output"][0]["type"], "message");
    }
}
