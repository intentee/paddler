use anyhow::Context as _;
use anyhow::Result;
use reqwest::Client;
use reqwest::StatusCode;
use url::Url;

pub async fn wait_until_balancer_healthy(management_base_url: &Url) -> Result<()> {
    let health_url = management_base_url
        .join("health")
        .context("failed to construct /health URL from management base URL")?;
    let client = Client::new();

    loop {
        match client.get(health_url.clone()).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => return Ok(()),
                StatusCode::SERVICE_UNAVAILABLE => {
                    tokio::task::yield_now().await;
                }
                other => {
                    return Err(anyhow::anyhow!(
                        "unexpected status {other} while probing {health_url}"
                    ));
                }
            },
            Err(request_error) => {
                if request_error.is_connect() {
                    tokio::task::yield_now().await;
                } else {
                    return Err(anyhow::Error::new(request_error)
                        .context(format!("failed to probe {health_url}")));
                }
            }
        }
    }
}
