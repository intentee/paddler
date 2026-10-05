use serde::Serialize;
use serde::Serializer;

use crate::compatibility::openai_service::responses_output_item::ResponsesOutputItem;
use crate::compatibility::openai_service::responses_response_header::ResponsesResponseHeader;
use crate::compatibility::openai_service::responses_response_progress::ResponsesResponseProgress;
use crate::compatibility::openai_service::responses_response_status::ResponsesResponseStatus;
use crate::compatibility::openai_service::responses_usage::ResponsesUsage;

fn serialized_progress(progress: &ResponsesResponseProgress) -> SerializedProgress<'_> {
    match progress {
        ResponsesResponseProgress::Completed { output, usage } => SerializedProgress {
            error: None,
            incomplete_details: None,
            output,
            status: ResponsesResponseStatus::Completed,
            usage: Some(*usage),
        },
        ResponsesResponseProgress::Failed { error_message } => SerializedProgress {
            error: Some(ResponseError {
                code: "server_error",
                message: error_message,
            }),
            incomplete_details: None,
            output: &[],
            status: ResponsesResponseStatus::Failed,
            usage: None,
        },
        ResponsesResponseProgress::Incomplete { output, usage } => SerializedProgress {
            error: None,
            incomplete_details: Some(IncompleteDetails {
                reason: "max_output_tokens",
            }),
            output,
            status: ResponsesResponseStatus::Incomplete,
            usage: Some(*usage),
        },
        ResponsesResponseProgress::InProgress => SerializedProgress {
            error: None,
            incomplete_details: None,
            output: &[],
            status: ResponsesResponseStatus::InProgress,
            usage: None,
        },
    }
}

#[derive(Serialize)]
struct ResponseError<'response> {
    code: &'static str,
    message: &'response str,
}

#[derive(Serialize)]
struct IncompleteDetails {
    reason: &'static str,
}

#[derive(Serialize)]
struct TextFormat {
    #[serde(rename = "type")]
    format_type: &'static str,
}

#[derive(Serialize)]
struct TextConfiguration {
    format: TextFormat,
}

#[derive(Serialize)]
struct EmptyMetadata {}

#[derive(Serialize)]
struct SerializedProgress<'response> {
    error: Option<ResponseError<'response>>,
    incomplete_details: Option<IncompleteDetails>,
    output: &'response [ResponsesOutputItem],
    status: ResponsesResponseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    usage: Option<ResponsesUsage>,
}

#[derive(Serialize)]
struct SerializedResponse<'response> {
    id: &'response str,
    object: &'static str,
    created_at: u64,
    instructions: Option<&'response str>,
    model: &'response str,
    tools: [u8; 0],
    parallel_tool_calls: bool,
    metadata: EmptyMetadata,
    tool_choice: &'static str,
    temperature: f32,
    top_p: f32,
    text: TextConfiguration,
    #[serde(flatten)]
    progress: SerializedProgress<'response>,
}

#[derive(Clone, Debug)]
pub struct ResponsesResponse {
    pub header: ResponsesResponseHeader,
    pub progress: ResponsesResponseProgress,
}

impl Serialize for ResponsesResponse {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        let Self {
            header:
                ResponsesResponseHeader {
                    created_at,
                    id,
                    instructions,
                    model,
                    temperature,
                    top_p,
                },
            progress,
        } = self;

        SerializedResponse {
            id,
            object: "response",
            created_at: *created_at,
            instructions: instructions.as_deref(),
            model,
            tools: [],
            parallel_tool_calls: true,
            metadata: EmptyMetadata {},
            tool_choice: "auto",
            temperature: *temperature,
            top_p: *top_p,
            text: TextConfiguration {
                format: TextFormat {
                    format_type: "text",
                },
            },
            progress: serialized_progress(progress),
        }
        .serialize(serializer)
    }
}
