use std::pin::Pin;

use futures_util::Stream;
use futures_util::StreamExt as _;
use reqwest::Client;
use reqwest::Response;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::from_str;
use tokio_util::sync::CancellationToken;
use url::Url;

use crate::error::Result;
use crate::format_api_url::format_api_url;
use crate::send_checked_request::send_checked_request;
use crate::stream::sse::Sse;

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

    pub async fn get_sse_json<TItem: DeserializeOwned + Send + 'static>(
        &self,
        cancellation_token: CancellationToken,
        path: &str,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<TItem>> + Send>>> {
        let response = self.get(cancellation_token.clone(), path).await?;
        let items = Sse::from_response(cancellation_token, response)
            .map(|data_result| data_result.and_then(|data| Ok(from_str(&data)?)));

        Ok(Box::pin(items))
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
    use futures_util::StreamExt as _;
    use http::StatusCode;
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use paddler_local_http_fixture::fixture_response::FixtureResponse;
    use paddler_local_http_fixture::local_http_fixture::LocalHttpFixture;
    use paddler_messaging::api_path::ApiPath;

    use super::HttpClient;
    use crate::error::Error;

    const UNREACHABLE_BASE_URL: &str = "http://127.0.0.1:1";

    fn unreachable_client() -> HttpClient {
        HttpClient::new(Url::parse(UNREACHABLE_BASE_URL).expect("the test URL must be valid"))
    }

    fn client_of(fixture: &LocalHttpFixture) -> HttpClient {
        HttpClient::new(Url::parse(&fixture.url("/")).expect("the fixture URL must be valid"))
    }

    async fn fixture_serving(response: FixtureResponse) -> LocalHttpFixture {
        LocalHttpFixture::start(response)
            .await
            .expect("the fixture server must start")
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
                .get(CancellationToken::new(), ApiPath::HEALTH)
                .await,
            Err(Error::Connect { url, .. }) if url == format!("{UNREACHABLE_BASE_URL}/health")
        ));
    }

    #[tokio::test]
    async fn a_cancelled_token_rejects_a_json_request() {
        assert!(matches!(
            unreachable_client()
                .get_json::<String>(cancelled_token(), ApiPath::AGENTS)
                .await,
            Err(Error::RequestCancelled { url }) if url == format!("{UNREACHABLE_BASE_URL}{}", ApiPath::AGENTS)
        ));
    }

    #[tokio::test]
    async fn a_cancelled_token_rejects_a_server_sent_event_request() {
        assert!(matches!(
            unreachable_client()
                .get_sse_json::<String>(cancelled_token(), ApiPath::AGENTS_STREAM)
                .await,
            Err(Error::RequestCancelled { url }) if url == format!("{UNREACHABLE_BASE_URL}{}", ApiPath::AGENTS_STREAM)
        ));
    }

    #[tokio::test]
    async fn a_cancelled_token_rejects_a_text_request() {
        assert!(matches!(
            unreachable_client()
                .get_text(cancelled_token(), ApiPath::METRICS)
                .await,
            Err(Error::RequestCancelled { url }) if url == format!("{UNREACHABLE_BASE_URL}/metrics")
        ));
    }

    #[tokio::test]
    async fn a_cancelled_token_rejects_a_post_request() {
        assert!(matches!(
            unreachable_client()
                .post_json(
                    cancelled_token(),
                    ApiPath::CONTINUE_FROM_RAW_PROMPT,
                    "body"
                )
                .await,
            Err(Error::RequestCancelled { url }) if url == format!("{UNREACHABLE_BASE_URL}{}", ApiPath::CONTINUE_FROM_RAW_PROMPT)
        ));
    }

    #[tokio::test]
    async fn a_cancelled_token_rejects_a_put_request() {
        assert!(matches!(
            unreachable_client()
                .put_json(cancelled_token(), ApiPath::BALANCER_DESIRED_STATE, "body")
                .await,
            Err(Error::RequestCancelled { url }) if url == format!("{UNREACHABLE_BASE_URL}{}", ApiPath::BALANCER_DESIRED_STATE)
        ));
    }

    #[tokio::test]
    async fn a_body_that_is_not_json_is_reported_as_undecodable() {
        let fixture = fixture_serving(FixtureResponse::Ok(b"not json".to_vec())).await;

        assert!(matches!(
            client_of(&fixture)
                .get_json::<String>(CancellationToken::new(), ApiPath::AGENTS)
                .await,
            Err(Error::Http(source)) if source.is_decode()
        ));
    }

    #[tokio::test]
    async fn a_server_sent_event_that_is_not_json_is_reported_as_undecodable() {
        let fixture = fixture_serving(FixtureResponse::Ok(b"data: not json\n\n".to_vec())).await;
        let mut events = client_of(&fixture)
            .get_sse_json::<String>(CancellationToken::new(), ApiPath::AGENTS_STREAM)
            .await
            .expect("the event stream must open");

        assert!(matches!(
            events.next().await,
            Some(Err(Error::Json(source))) if source.is_syntax()
        ));
    }

    #[tokio::test]
    async fn a_truncated_text_body_is_reported_as_unreadable() {
        let fixture = fixture_serving(FixtureResponse::TruncatedBody {
            sent_body: b"paddler_".to_vec(),
            status: StatusCode::OK,
            withheld_byte_count: 8,
        })
        .await;

        assert!(matches!(
            client_of(&fixture)
                .get_text(CancellationToken::new(), ApiPath::METRICS)
                .await,
            Err(Error::Http(source)) if source.is_decode()
        ));
    }
}
