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
    use reqwest::Client;
    use tokio_util::sync::CancellationToken;

    use super::send_checked_request;
    use crate::error::Error;

    #[tokio::test]
    async fn a_refused_connection_maps_to_the_connect_variant() {
        let url = "http://127.0.0.1:1/health".to_owned();
        let request_builder = Client::new().get(&url);

        assert!(matches!(
            send_checked_request(CancellationToken::new(), url, request_builder).await,
            Err(Error::Connect { .. })
        ));
    }

    #[tokio::test]
    async fn an_already_cancelled_token_rejects_the_request_without_sending_it() {
        let url = "http://127.0.0.1:1/health".to_owned();
        let request_builder = Client::new().get(&url);
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        assert!(matches!(
            send_checked_request(cancellation_token, url, request_builder).await,
            Err(Error::RequestCancelled { .. })
        ));
    }
}
