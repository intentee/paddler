use reqwest::Client;
use reqwest::Response;
use serde::Serialize;
use serde::de::DeserializeOwned;
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

    pub async fn get(&self, path: &str) -> Result<Response> {
        let api_url = format_api_url(&self.url, path);
        let request_builder = self.reqwest_client.get(&api_url);

        send_checked_request(api_url, request_builder).await
    }

    pub async fn get_json<TResponse: DeserializeOwned>(&self, path: &str) -> Result<TResponse> {
        Ok(self.get(path).await?.json().await?)
    }

    pub async fn get_text(&self, path: &str) -> Result<String> {
        Ok(self.get(path).await?.text().await?)
    }

    pub async fn post_json<TBody: Serialize + Sync + ?Sized>(
        &self,
        path: &str,
        body: &TBody,
    ) -> Result<Response> {
        let api_url = format_api_url(&self.url, path);
        let request_builder = self.reqwest_client.post(&api_url).json(body);

        send_checked_request(api_url, request_builder).await
    }

    pub async fn put_json<TBody: Serialize + Sync + ?Sized>(
        &self,
        path: &str,
        body: &TBody,
    ) -> Result<Response> {
        let api_url = format_api_url(&self.url, path);
        let request_builder = self.reqwest_client.put(&api_url).json(body);

        send_checked_request(api_url, request_builder).await
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::json;
    use tokio::io::AsyncReadExt as _;
    use tokio::io::AsyncWriteExt as _;
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use url::Url;

    use super::HttpClient;
    use crate::error::Error;

    struct RecordedRequest {
        head: String,
    }

    async fn serve_one_request(
        raw_response: &'static str,
    ) -> (HttpClient, oneshot::Receiver<RecordedRequest>) {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("the fixture server must bind a loopback port");
        let address = listener
            .local_addr()
            .expect("the fixture server must report its address");
        let (recorded_request_tx, recorded_request_rx) = oneshot::channel();

        tokio::spawn(async move {
            let (mut connection, _peer_address) = listener
                .accept()
                .await
                .expect("the fixture server must accept the connection");
            let mut request_bytes = [0_u8; 1024];
            let request_byte_count = connection
                .read(&mut request_bytes)
                .await
                .expect("the fixture server must read the request");

            connection
                .write_all(raw_response.as_bytes())
                .await
                .expect("the fixture server must write the response");

            let head = String::from_utf8_lossy(&request_bytes[..request_byte_count]).into_owned();

            recorded_request_tx
                .send(RecordedRequest { head })
                .unwrap_or_else(|_recorded_request| panic!("the request must be recorded"));
        });

        let url =
            Url::parse(&format!("http://{address}")).expect("the fixture server URL must be valid");

        (HttpClient::new(url), recorded_request_rx)
    }

    fn unreachable_client() -> HttpClient {
        HttpClient::new(Url::parse("http://127.0.0.1:1").expect("the test URL must be valid"))
    }

    #[tokio::test]
    async fn get_text_returns_the_response_body() {
        let (http_client, _recorded_request_rx) =
            serve_one_request("HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK").await;

        assert_eq!(
            http_client
                .get_text("/health")
                .await
                .expect("the health body must be returned"),
            "OK"
        );
    }

    #[tokio::test]
    async fn get_json_deserializes_the_response_body() {
        let (http_client, recorded_request_rx) = serve_one_request(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 13\r\n\r\n{\"agents\":[]}",
        )
        .await;

        let deserialized: Value = http_client
            .get_json("/api/v1/agents")
            .await
            .expect("the response body must deserialize");

        assert_eq!(deserialized, json!({ "agents": [] }));

        let recorded_request = recorded_request_rx
            .await
            .expect("the fixture server must record the request");

        assert!(recorded_request.head.starts_with("GET /api/v1/agents "));
    }

    #[tokio::test]
    async fn post_json_sends_the_body_to_the_formatted_path() {
        let (http_client, recorded_request_rx) =
            serve_one_request("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").await;

        http_client
            .post_json("/api/v1/continue_from_raw_prompt", &json!({ "a": 1 }))
            .await
            .expect("the request must succeed");

        let recorded_request = recorded_request_rx
            .await
            .expect("the fixture server must record the request");

        assert!(
            recorded_request
                .head
                .starts_with("POST /api/v1/continue_from_raw_prompt ")
        );
        assert!(recorded_request.head.ends_with("{\"a\":1}"));
    }

    #[tokio::test]
    async fn put_json_sends_the_body_to_the_formatted_path() {
        let (http_client, recorded_request_rx) =
            serve_one_request("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").await;

        http_client
            .put_json("/api/v1/balancer_desired_state", &json!({ "a": 1 }))
            .await
            .expect("the request must succeed");

        let recorded_request = recorded_request_rx
            .await
            .expect("the fixture server must record the request");

        assert!(
            recorded_request
                .head
                .starts_with("PUT /api/v1/balancer_desired_state ")
        );
        assert!(recorded_request.head.ends_with("{\"a\":1}"));
    }

    #[tokio::test]
    async fn an_unreachable_server_maps_to_the_connect_variant() {
        assert!(matches!(
            unreachable_client().get("/health").await,
            Err(Error::Connect { .. })
        ));
    }
}
