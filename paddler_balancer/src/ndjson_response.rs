use actix_web::Error;
use actix_web::HttpResponse;
use actix_web::http::header;
use futures::stream::Stream;
use futures::stream::StreamExt as _;

use crate::chunk_forwarding_session_controller::transform_result::TransformResult;

pub fn ndjson_response<TStream>(transform_results: TStream) -> HttpResponse
where
    TStream: Stream<Item = TransformResult> + 'static,
{
    HttpResponse::Ok()
        .insert_header(header::ContentType::json())
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .streaming(transform_results.filter_map(|transform_result| async move {
            transform_result.into_ndjson_line().map(Ok::<_, Error>)
        }))
}

#[cfg(test)]
mod tests {
    use actix_web::body;
    use futures::stream::iter;

    use super::ndjson_response;
    use crate::chunk_forwarding_session_controller::transform_result::TransformResult;

    #[actix_web::test]
    async fn writes_every_chunk_and_error_as_a_line_and_skips_discarded_results() {
        let response = ndjson_response(iter([
            TransformResult::Chunk("{\"token\":\"hi\"}".to_owned()),
            TransformResult::Discard,
            TransformResult::Error("boom".to_owned()),
        ]));

        let body_bytes = body::to_bytes(response.into_body()).await.unwrap();

        assert_eq!(body_bytes.as_ref(), b"{\"token\":\"hi\"}\nboom\n");
    }
}
