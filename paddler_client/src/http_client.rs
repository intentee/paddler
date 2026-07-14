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
    use url::Url;

    use super::HttpClient;
    use crate::error::Error;

    fn unreachable_client() -> HttpClient {
        HttpClient::new(Url::parse("http://127.0.0.1:1").expect("the test URL must be valid"))
    }

    #[tokio::test]
    async fn an_unreachable_server_maps_to_the_connect_variant() {
        assert!(matches!(
            unreachable_client().get("/health").await,
            Err(Error::Connect { .. })
        ));
    }
}
