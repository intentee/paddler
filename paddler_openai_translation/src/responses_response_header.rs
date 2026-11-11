use paddler_messaging::generation_finish::GenerationFinish;
use paddler_messaging::generation_summary::GenerationSummary;
use paddler_messaging::token_usage::TokenUsage;

use crate::arguments_to_tool_call_string::arguments_to_tool_call_string;
use crate::completes_generation::CompletesGeneration;
use crate::generated_output::GeneratedOutput;
use crate::responses_output_item::ResponsesOutputItem;
use crate::responses_output_item_kind::ResponsesOutputItemKind;
use crate::responses_response::ResponsesResponse;
use crate::responses_response_progress::ResponsesResponseProgress;
use crate::responses_usage::ResponsesUsage;

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
    pub fn failed(&self, error_message: String) -> ResponsesResponse {
        self.with_progress(ResponsesResponseProgress::Failed { error_message })
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
    pub fn in_progress(&self) -> ResponsesResponse {
        self.with_progress(ResponsesResponseProgress::InProgress)
    }

    fn with_progress(&self, progress: ResponsesResponseProgress) -> ResponsesResponse {
        let Self {
            created_at,
            id,
            instructions,
            model,
            temperature,
            top_p,
        } = self.clone();

        ResponsesResponse {
            created_at,
            id,
            instructions,
            model,
            progress,
            temperature,
            top_p,
        }
    }
}

impl CompletesGeneration for ResponsesResponseHeader {
    type Completion = ResponsesResponse;

    fn complete(
        &self,
        _request_id: &str,
        GeneratedOutput {
            content,
            reasoning,
            tool_calls,
        }: GeneratedOutput,
        GenerationSummary { finish, usage }: &GenerationSummary,
    ) -> ResponsesResponse {
        let mut output: Vec<ResponsesOutputItem> = Vec::new();

        if !reasoning.is_empty() {
            output.push(ResponsesOutputItem::completed_reasoning(
                ResponsesOutputItemKind::Reasoning.item_id(),
                reasoning,
            ));
        }

        if !content.is_empty() || tool_calls.is_empty() {
            output.push(ResponsesOutputItem::completed_message(
                ResponsesOutputItemKind::Message.item_id(),
                content,
            ));
        }

        for parsed_call in &tool_calls {
            output.push(ResponsesOutputItem::completed_function_call(
                ResponsesOutputItemKind::FunctionCall.item_id(),
                parsed_call,
                arguments_to_tool_call_string(&parsed_call.arguments),
            ));
        }

        self.finished(output, usage, *finish)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::json;
    use serde_json::to_value;

    use paddler_generation_result_fixture::weather_call::weather_call;
    use paddler_messaging::generation_finish::GenerationFinish;
    use paddler_messaging::generation_summary::GenerationSummary;
    use paddler_messaging::token_usage::TokenUsage;
    use paddler_openai_response_format_validator::openai_validator::OpenAIValidator;

    use super::ResponsesResponseHeader;
    use crate::completes_generation::CompletesGeneration as _;
    use crate::generated_output::GeneratedOutput;
    use crate::generated_output_part::GeneratedOutputPart;
    use crate::responses_output_item::ResponsesOutputItem;

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

    fn completed_response_of(generated_output_parts: Vec<GeneratedOutputPart>) -> Value {
        let mut generated_output = GeneratedOutput::default();

        for generated_output_part in generated_output_parts {
            generated_output.append(generated_output_part);
        }

        to_value(header().complete(
            "test-request",
            generated_output,
            &GenerationSummary {
                finish: GenerationFinish::EndOfGeneration,
                usage: TokenUsage {
                    prompt: 5,
                    content: 3,
                    reasoning: 2,
                    ..TokenUsage::default()
                },
            },
        ))
        .unwrap()
    }

    #[test]
    fn in_progress_reports_in_progress_status_with_no_output() {
        let response = to_value(header().in_progress()).unwrap();

        assert_eq!(response.pointer("/status"), Some(&json!("in_progress")));
        assert_eq!(response.pointer("/object"), Some(&json!("response")));
        assert_eq!(response.pointer("/output"), Some(&json!([])));
        assert!(response.get("usage").is_none());
    }

    #[test]
    fn reports_the_sampling_parameters_in_effect() {
        let response = to_value(header().in_progress()).unwrap();

        assert_eq!(response.pointer("/temperature"), Some(&json!(0.25)));
        assert_eq!(response.pointer("/top_p"), Some(&json!(0.5)));
    }

    #[test]
    fn failed_carries_the_error_message() {
        let response = to_value(header().failed("boom".to_owned())).unwrap();

        assert_eq!(response.pointer("/status"), Some(&json!("failed")));
        assert_eq!(response.pointer("/error/message"), Some(&json!("boom")));
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
                prompt: 3,
                cached_prompt: 0,
                input_image: 0,
                input_audio: 0,
                content: 5,
                reasoning: 2,
                tool_call: 0,
                undeterminable: 0,
            },
            GenerationFinish::EndOfGeneration,
        ))
        .unwrap();

        assert_eq!(response.pointer("/status"), Some(&json!("completed")));
        assert_eq!(response.pointer("/usage/input_tokens"), Some(&json!(3)));
        assert_eq!(response.pointer("/output/0/type"), Some(&json!("message")));
    }

    #[test]
    fn completes_content_into_a_message_item() {
        let response = completed_response_of(vec![
            GeneratedOutputPart::Content("hel".to_owned()),
            GeneratedOutputPart::Content("lo".to_owned()),
        ]);

        assert_eq!(response.pointer("/id"), Some(&json!("resp_test")));
        assert_eq!(response.pointer("/status"), Some(&json!("completed")));
        assert_eq!(response.pointer("/output/0/type"), Some(&json!("message")));
        assert_eq!(
            response.pointer("/output/0/content/0/text"),
            Some(&json!("hello"))
        );
    }

    #[test]
    fn two_responses_never_share_an_output_item_id() {
        let first_response =
            completed_response_of(vec![GeneratedOutputPart::Content("hello".to_owned())]);
        let second_response =
            completed_response_of(vec![GeneratedOutputPart::Content("hello".to_owned())]);

        assert_ne!(
            first_response.pointer("/output/0/id"),
            second_response.pointer("/output/0/id")
        );
    }

    #[test]
    fn completes_reasoning_and_tool_calls_without_an_empty_message() {
        let response = completed_response_of(vec![
            GeneratedOutputPart::Reasoning("ponder".to_owned()),
            GeneratedOutputPart::ToolCalls(vec![weather_call()]),
        ]);

        assert_eq!(
            response.pointer("/output/0/type"),
            Some(&json!("reasoning"))
        );
        assert_eq!(
            response.pointer("/output/1/type"),
            Some(&json!("function_call"))
        );
        assert_eq!(
            response.pointer("/output/1/name"),
            Some(&json!("get_weather"))
        );
        assert_eq!(
            response.pointer("/usage/output_tokens_details/reasoning_tokens"),
            Some(&json!(2))
        );
    }

    #[test]
    fn the_completed_response_conforms_to_the_schema() {
        OpenAIValidator::new()
            .unwrap()
            .validate_responses_response(&completed_response_of(vec![
                GeneratedOutputPart::Reasoning("p".to_owned()),
                GeneratedOutputPart::Content("hello".to_owned()),
                GeneratedOutputPart::ToolCalls(vec![weather_call()]),
            ]))
            .unwrap();
    }
}
