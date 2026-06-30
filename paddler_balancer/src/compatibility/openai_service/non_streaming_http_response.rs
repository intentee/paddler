use actix_web::HttpResponse;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;
use crate::compatibility::openai_service::openai_error::OpenAIError;

#[must_use]
pub fn non_streaming_http_response(
    results: Vec<TransformResult>,
    no_result_message: &'static str,
) -> HttpResponse {
    let mut response_body: Option<String> = None;

    for result in results {
        match result {
            TransformResult::Error(error_json) => {
                return HttpResponse::InternalServerError()
                    .content_type("application/json")
                    .body(error_json);
            }
            TransformResult::Chunk(content) => {
                response_body = Some(content);
            }
            TransformResult::Discard => {}
        }
    }

    response_body.map_or_else(
        || {
            HttpResponse::InternalServerError()
                .content_type("application/json")
                .body(
                    OpenAIError {
                        error_type: "server_error",
                        message: no_result_message.to_owned(),
                    }
                    .to_envelope()
                    .to_string(),
                )
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
    use actix_web::HttpResponse;
    use actix_web::body::to_bytes;
    use actix_web::http::StatusCode;

    use super::non_streaming_http_response;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;

    async fn body_string(response: HttpResponse) -> String {
        let bytes = to_bytes(response.into_body()).await.unwrap();

        String::from_utf8(bytes.to_vec()).unwrap()
    }

    #[actix_web::test]
    async fn a_chunk_becomes_a_successful_response() {
        let response = non_streaming_http_response(
            vec![TransformResult::Chunk("{\"ok\":true}".to_owned())],
            "no result",
        );

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_string(response).await, "{\"ok\":true}");
    }

    #[actix_web::test]
    async fn an_error_becomes_an_internal_server_error_with_its_body() {
        let response = non_streaming_http_response(
            vec![TransformResult::Error("{\"error\":\"boom\"}".to_owned())],
            "no result",
        );

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body_string(response).await, "{\"error\":\"boom\"}");
    }

    #[actix_web::test]
    async fn an_error_takes_priority_over_a_preceding_chunk() {
        let response = non_streaming_http_response(
            vec![
                TransformResult::Chunk("{\"ok\":true}".to_owned()),
                TransformResult::Error("{\"error\":\"boom\"}".to_owned()),
            ],
            "no result",
        );

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body_string(response).await, "{\"error\":\"boom\"}");
    }

    #[actix_web::test]
    async fn discarded_results_are_skipped_before_the_chunk() {
        let response = non_streaming_http_response(
            vec![
                TransformResult::Discard,
                TransformResult::Chunk("{\"ok\":true}".to_owned()),
            ],
            "no result",
        );

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_string(response).await, "{\"ok\":true}");
    }

    #[actix_web::test]
    async fn no_terminal_result_produces_the_no_result_error() {
        let response = non_streaming_http_response(vec![TransformResult::Discard], "no completion");

        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let parsed: serde_json::Value = serde_json::from_str(&body_string(response).await).unwrap();

        assert_eq!(parsed["error"]["type"], "server_error");
        assert_eq!(parsed["error"]["message"], "no completion");
    }
}
