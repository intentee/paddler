use std::error::Error as _;
use std::io::ErrorKind;
use std::time::Duration;

use reqwest::Client;
use reqwest::StatusCode;
use url::Url;

use crate::error::ClusterError;

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

pub async fn wait_until_healthy(
    base_url: &Url,
    endpoint: &str,
) -> std::result::Result<(), ClusterError> {
    let health_url =
        base_url
            .join(endpoint)
            .map_err(|source| ClusterError::ProbeUrlConstruction {
                endpoint: endpoint.to_owned(),
                base_url: base_url.clone(),
                source,
            })?;
    let client = Client::new();

    loop {
        match client.get(health_url.clone()).send().await {
            Ok(response) => match response.status() {
                StatusCode::OK => return Ok(()),
                StatusCode::SERVICE_UNAVAILABLE => {
                    tokio::time::sleep(HEALTHCHECK_PROBE_INTERVAL).await;
                }
                other => {
                    return Err(ClusterError::ProbeUnexpectedStatus {
                        status: other.as_u16(),
                        url: health_url,
                    });
                }
            },
            Err(request_error) => {
                if is_transient_probe_error(&request_error) {
                    tokio::time::sleep(HEALTHCHECK_PROBE_INTERVAL).await;
                } else {
                    return Err(ClusterError::ProbeFailed {
                        url: health_url,
                        source: request_error,
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use url::Url;

    use super::wait_until_healthy;
    use crate::error::ClusterError;

    #[tokio::test]
    async fn fails_to_construct_the_probe_url_for_a_malformed_endpoint() -> Result<()> {
        let base_url = Url::parse("http://127.0.0.1:8080/")?;

        let outcome = wait_until_healthy(&base_url, "http://").await;

        assert!(matches!(
            outcome,
            Err(ClusterError::ProbeUrlConstruction { .. })
        ));

        Ok(())
    }
}
