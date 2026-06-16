use std::error::Error as _;
use std::io::ErrorKind;
use std::time::Duration;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use reqwest::Client;
use reqwest::StatusCode;
use url::Url;

const HEALTHCHECK_PROBE_INTERVAL: Duration = Duration::from_millis(20);

fn is_transient_probe_error(request_error: &reqwest::Error) -> bool {
    if request_error.is_connect() || request_error.is_timeout() {
        return true;
    }

    let mut source = request_error.source();

    while let Some(error) = source {
        if let Some(io_error) = error.downcast_ref::<std::io::Error>() {
            return matches!(
                io_error.kind(),
                ErrorKind::ConnectionReset
                    | ErrorKind::ConnectionRefused
                    | ErrorKind::ConnectionAborted
                    | ErrorKind::BrokenPipe
            );
        }

        source = error.source();
    }

    false
}

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
                    return Err(anyhow!(
                        "unexpected status {other} while probing {health_url}"
                    ));
                }
            },
            Err(request_error) => {
                if is_transient_probe_error(&request_error) {
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
