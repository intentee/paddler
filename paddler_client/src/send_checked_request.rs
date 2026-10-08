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
        Ok(response) if response.status().is_success() => Ok(response),
        Ok(response) => Err(rejection(response, url).await?),
        Err(source) if source.is_connect() => Err(Error::Connect { url, source }),
        Err(source) => Err(Error::Http(source)),
    }
}

#[cfg(test)]
mod tests {
    use http::StatusCode;
    use reqwest::Client;
    use reqwest::Response;
    use tokio_util::sync::CancellationToken;

    use paddler_local_http_fixture::fixture_response::FixtureResponse;
    use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
    use paddler_messaging::api_path::ApiPath;

    use super::send_checked_request;
    use crate::error::Error;
    use crate::error::Result;

    const UNREACHABLE_HEALTH_URL: &str = "http://127.0.0.1:1/health";

    async fn response_from(fixture_response: FixtureResponse) -> Result<Response> {
        let fixture = LocalHttpFixture::start(fixture_response)
            .await
            .expect("the fixture server must start");
        let url = fixture.url(ApiPath::HEALTH);
        let request_builder = Client::new().get(&url);

        send_checked_request(CancellationToken::new(), url, request_builder).await
    }

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
        assert!(matches!(
            response_from(FixtureResponse::TruncatedBody {
                sent_body: b"inter".to_vec(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
                withheld_byte_count: 5,
            })
            .await,
            Err(Error::Http(source)) if source.is_decode()
        ));
    }
}
