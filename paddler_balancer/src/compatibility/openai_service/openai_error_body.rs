use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use serde::Serialize;
use serde::Serializer;

use crate::compatibility::openai_service::openai_error_type::OpenAIErrorType;

#[derive(Serialize)]
struct SerializedError<'body> {
    message: &'body str,
    #[serde(rename = "type")]
    error_type: OpenAIErrorType,
    param: (),
    code: (),
}

#[derive(Serialize)]
struct SerializedErrorBody<'body> {
    error: SerializedError<'body>,
}

pub struct OpenAIErrorBody {
    pub error_type: OpenAIErrorType,
    pub message: String,
}

impl OpenAIErrorBody {
    #[must_use]
    pub fn into_http_response(self, status_code: StatusCode) -> HttpResponse {
        HttpResponse::build(status_code).json(self)
    }
}

impl Serialize for OpenAIErrorBody {
    fn serialize<TSerializer>(
        &self,
        serializer: TSerializer,
    ) -> Result<TSerializer::Ok, TSerializer::Error>
    where
        TSerializer: Serializer,
    {
        SerializedErrorBody {
            error: SerializedError {
                message: &self.message,
                error_type: self.error_type,
                param: (),
                code: (),
            },
        }
        .serialize(serializer)
    }
}
