use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use reqwest::Client;
use reqwest::StatusCode;
use url::Url;

const HEALTHCHECK_PROBE_INTERVAL: Duration = Duration::from_millis(20);

pub async fn wait_until_healthy(base_url: &Url, endpoint: &str) -> Result<()> {
    let health_url = base_url
        .join(endpoint)
        .with_context(|| format!("failed to construct {endpoint} URL from {base_url}"))?;
    let client = Client::new();

    loop {
        match client.get(health_url.clone()).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => return Ok(()),
                StatusCode::SERVICE_UNAVAILABLE => {
                    tokio::time::sleep(HEALTHCHECK_PROBE_INTERVAL).await;
                }
                other => {
                    return Err(anyhow::anyhow!(
                        "unexpected status {other} while probing {health_url}"
                    ));
                }
            },
            Err(request_error) => {
                if request_error.is_connect() {
                    tokio::time::sleep(HEALTHCHECK_PROBE_INTERVAL).await;
                } else {
                    return Err(anyhow::Error::new(request_error)
                        .context(format!("failed to probe {health_url}")));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::wait_until_healthy;

    #[tokio::test]
    async fn fails_to_construct_the_probe_url_for_a_malformed_endpoint() {
        let base_url = Url::parse("http://127.0.0.1:8080/").unwrap();

        let error = wait_until_healthy(&base_url, "http://")
            .await
            .err()
            .unwrap();

        assert!(error.to_string().contains("failed to construct"));
    }
}
