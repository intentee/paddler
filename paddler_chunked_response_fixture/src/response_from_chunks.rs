use std::io::Result as IoResult;

use futures_util::stream::iter;
use http::Response as HttpResponse;
use reqwest::Body;
use reqwest::Response;

#[must_use]
pub fn response_from_chunks(chunks: Vec<IoResult<&'static [u8]>>) -> Response {
    let stream = iter(chunks.into_iter().map(|chunk| chunk.map(<[u8]>::to_vec)));

    Response::from(HttpResponse::new(Body::wrap_stream(stream)))
}
