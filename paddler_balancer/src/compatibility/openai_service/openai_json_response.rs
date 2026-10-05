use actix_web::HttpResponse;
use actix_web::http::StatusCode;
use futures::stream::Stream;
use futures::stream::StreamExt as _;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::compatibility::openai_service::openai_error::OpenAIError;
use crate::compatibility::openai_service::openai_error_type::OpenAIErrorType;

pub async fn openai_json_response<TStream>(
    transform_results: TStream,
    missing_body_message: &str,
) -> HttpResponse
where
    TStream: Stream<Item = TransformResult>,
{
    let mut json_bodies: Vec<String> = Vec::new();

    for transform_result in transform_results.collect::<Vec<TransformResult>>().await {
        match transform_result {
            TransformResult::Chunk(json_body) => json_bodies.push(json_body),
            TransformResult::Discard => {}
            TransformResult::Error(error_json) => {
                return HttpResponse::InternalServerError()
                    .content_type("application/json")
                    .body(error_json);
            }
        }
    }

    json_bodies.into_iter().next().map_or_else(
        || {
            OpenAIError {
                error_type: OpenAIErrorType::ServerError,
                message: missing_body_message.to_owned(),
            }
            .to_http_response(StatusCode::INTERNAL_SERVER_ERROR)
        },
        |json_body| {
            HttpResponse::Ok()
                .content_type("application/json")
                .body(json_body)
        },
    )
}

#[cfg(test)]
mod tests {
    use actix_web::body::to_bytes;
    use actix_web::http::StatusCode;
    use futures::stream::iter;
    use serde_json::Value;
    use serde_json::from_slice;
    use serde_json::json;

    use super::openai_json_response;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;

    #[actix_web::test]
    async fn answers_with_the_completion_that_follows_discarded_results() {
        let response = openai_json_response(
            iter([
                TransformResult::Discard,
                TransformResult::Chunk(r#"{"id":"completion"}"#.to_owned()),
            ]),
            "no completion",
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            to_bytes(response.into_body()).await.unwrap(),
            r#"{"id":"completion"}"#
        );
    }

    #[actix_web::test]
    async fn answers_with_the_error_even_after_a_completion() {
        let response = openai_json_response(
            iter([
                TransformResult::Chunk(r#"{"id":"completion"}"#.to_owned()),
                TransformResult::Error(r#"{"error":"agent failed"}"#.to_owned()),
            ]),
            "no completion",
        )
        .await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            to_bytes(response.into_body()).await.unwrap(),
            r#"{"error":"agent failed"}"#
        );
    }

    #[actix_web::test]
    async fn answers_a_server_error_when_no_completion_was_produced() {
        let response =
            openai_json_response(iter([TransformResult::Discard]), "no completion").await;

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            from_slice::<Value>(&to_bytes(response.into_body()).await.unwrap()).unwrap(),
            json!({
                "error": {
                    "message": "no completion",
                    "type": "server_error",
                    "param": null,
                    "code": null
                }
            })
        );
    }
}
