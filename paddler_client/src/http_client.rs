use reqwest::Client;
use reqwest::Response;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::error::Result;
use crate::format_api_url::format_api_url;
use crate::send_checked_request::send_checked_request;

#[derive(Clone)]
pub struct HttpClient {
    reqwest_client: Client,
    url: Url,
}

impl HttpClient {
    #[must_use]
    pub fn new(url: Url) -> Self {
        Self {
            reqwest_client: Client::new(),
            url,
        }
    }

    pub async fn get(&self, cancellation_token: CancellationToken, path: &str) -> Result<Response> {
        let api_url = format_api_url(&self.url, path);
        let request_builder = self.reqwest_client.get(&api_url);

        send_checked_request(cancellation_token, api_url, request_builder).await
    }

    pub async fn get_json<TResponse: DeserializeOwned>(
        &self,
        cancellation_token: CancellationToken,
        path: &str,
    ) -> Result<TResponse> {
        Ok(self.get(cancellation_token, path).await?.json().await?)
    }

    pub async fn get_text(
        &self,
        cancellation_token: CancellationToken,
        path: &str,
    ) -> Result<String> {
        Ok(self.get(cancellation_token, path).await?.text().await?)
    }

    pub async fn post_json<TBody: Serialize + Sync + ?Sized>(
        &self,
        cancellation_token: CancellationToken,
        path: &str,
        body: &TBody,
    ) -> Result<Response> {
        let api_url = format_api_url(&self.url, path);
        let request_builder = self.reqwest_client.post(&api_url).json(body);

        send_checked_request(cancellation_token, api_url, request_builder).await
    }

    pub async fn put_json<TBody: Serialize + Sync + ?Sized>(
        &self,
        cancellation_token: CancellationToken,
        path: &str,
        body: &TBody,
    ) -> Result<Response> {
        let api_url = format_api_url(&self.url, path);
        let request_builder = self.reqwest_client.put(&api_url).json(body);

        send_checked_request(cancellation_token, api_url, request_builder).await
    }
}

#[cfg(test)]
mod tests {
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::HttpClient;
    use crate::error::Error;

    fn unreachable_client() -> HttpClient {
        HttpClient::new(Url::parse("http://127.0.0.1:1").expect("the test URL must be valid"))
    }

    fn cancelled_token() -> CancellationToken {
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        cancellation_token
    }

    #[tokio::test]
    async fn an_unreachable_server_maps_to_the_connect_variant() {
        assert!(matches!(
            unreachable_client()
                .get(CancellationToken::new(), "/health")
                .await,
            Err(Error::Connect { .. })
        ));
    }

    #[tokio::test]
    async fn a_cancelled_token_rejects_a_json_request() {
        assert!(matches!(
            unreachable_client()
                .get_json::<String>(cancelled_token(), "/api/v1/agents")
                .await,
            Err(Error::RequestCancelled { .. })
        ));
    }

    #[tokio::test]
    async fn a_cancelled_token_rejects_a_text_request() {
        assert!(matches!(
            unreachable_client()
                .get_text(cancelled_token(), "/metrics")
                .await,
            Err(Error::RequestCancelled { .. })
        ));
    }

    #[tokio::test]
    async fn a_cancelled_token_rejects_a_post_request() {
        assert!(matches!(
            unreachable_client()
                .post_json(
                    cancelled_token(),
                    "/api/v1/continue_from_raw_prompt",
                    "body"
                )
                .await,
            Err(Error::RequestCancelled { .. })
        ));
    }

    #[tokio::test]
    async fn a_cancelled_token_rejects_a_put_request() {
        assert!(matches!(
            unreachable_client()
                .put_json(cancelled_token(), "/api/v1/balancer_desired_state", "body")
                .await,
            Err(Error::RequestCancelled { .. })
        ));
    }
}
