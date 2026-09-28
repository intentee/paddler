use reqwest::Client;
use reqwest::Method;
use reqwest::Response;
use reqwest::Url;

pub async fn send_cors_preflight(url: Url, origin: &str) -> reqwest::Result<Response> {
    Client::new()
        .request(Method::OPTIONS, url)
        .header("Origin", origin)
        .header("Access-Control-Request-Method", "GET")
        .send()
        .await
}
