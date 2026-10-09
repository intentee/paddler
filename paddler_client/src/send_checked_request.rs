use reqwest::RequestBuilder;
use reqwest::Response;
use reqwest::StatusCode;
use tokio_util::sync::CancellationToken;

use crate::error::Error;
use crate::error::Result;

async fn rejection(response: Response, url: String) -> Result<Error> {
    let status = response.status();
    let message = response.text().await?;

    if status == StatusCode::SERVICE_UNAVAILABLE {
        Ok(Error::ServiceUnavailable { message, url })
    } else {
        Ok(Error::UnexpectedResponseStatus {
            message,
            status,
            url,
        })
    }
}

async fn checked_response(response: Response, url: String) -> Result<Response> {
    if response.status().is_success() {
        return Ok(response);
    }

    Err(rejection(response, url).await?)
}

pub async fn send_checked_request(
    cancellation_token: CancellationToken,
    url: String,
    request_builder: RequestBuilder,
) -> Result<Response> {
    let Some(send_result) = cancellation_token
        .run_until_cancelled(request_builder.send())
        .await
    else {
        return Err(Error::RequestCancelled { url });
    };

    match send_result {
        Ok(response) => checked_response(response, url).await,
        Err(source) if source.is_connect() => Err(Error::Connect { url, source }),
        Err(source) => Err(Error::Http(source)),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Error as IoError;
    use std::io::ErrorKind;
    use std::io::Result as IoResult;

    use futures_util::stream::iter;
    use http::Response as HttpResponse;
    use http::StatusCode;
    use reqwest::Body;
    use reqwest::Client;
    use tokio_util::sync::CancellationToken;

    use super::checked_response;
    use super::send_checked_request;
    use crate::error::Error;

    const UNREACHABLE_HEALTH_URL: &str = "http://127.0.0.1:1/health";

    #[tokio::test]
    async fn a_refused_connection_maps_to_the_connect_variant() {
        let request_builder = Client::new().get(UNREACHABLE_HEALTH_URL);

        assert!(matches!(
            send_checked_request(
                CancellationToken::new(),
                UNREACHABLE_HEALTH_URL.to_owned(),
                request_builder
            )
            .await,
            Err(Error::Connect { url, .. }) if url == UNREACHABLE_HEALTH_URL
        ));
    }

    #[tokio::test]
    async fn an_already_cancelled_token_rejects_the_request_without_sending_it() {
        let request_builder = Client::new().get(UNREACHABLE_HEALTH_URL);
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        assert!(matches!(
            send_checked_request(
                cancellation_token,
                UNREACHABLE_HEALTH_URL.to_owned(),
                request_builder
            )
            .await,
            Err(Error::RequestCancelled { url }) if url == UNREACHABLE_HEALTH_URL
        ));
    }

    #[tokio::test]
    async fn a_rejection_whose_body_cannot_be_read_is_reported_as_undecodable() {
        let chunks: [IoResult<Vec<u8>>; 2] = [
            Ok(b"inter".to_vec()),
            Err(IoError::from(ErrorKind::UnexpectedEof)),
        ];
        let rejection = HttpResponse::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Body::wrap_stream(iter(chunks)))
            .expect("the rejection must be buildable");

        assert!(matches!(
            checked_response(rejection.into(), UNREACHABLE_HEALTH_URL.to_owned()).await,
            Err(Error::Http(source)) if source.is_decode()
        ));
    }
}
