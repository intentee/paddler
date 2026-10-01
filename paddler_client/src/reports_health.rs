use std::future::Future;
use std::time::Duration;

use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use paddler_messaging::api_path::ApiPath;

use crate::error::Error;
use crate::error::Result;
use crate::http_client::HttpClient;

const HEALTHCHECK_PROBE_INTERVAL: Duration = Duration::from_millis(20);

pub trait ReportsHealth: Sync {
    fn http_client(&self) -> &HttpClient;

    fn get_health(
        &self,
        cancellation_token: CancellationToken,
    ) -> impl Future<Output = Result<String>> + Send {
        async move {
            self.http_client()
                .get_text(cancellation_token, ApiPath::HEALTH)
                .await
        }
    }

    fn wait_until_healthy(
        &self,
        cancellation_token: CancellationToken,
    ) -> impl Future<Output = Result<()>> + Send {
        async move {
            loop {
                match self.get_health(cancellation_token.clone()).await {
                    Ok(_health_body) => return Ok(()),
                    Err(Error::Connect { .. }) => {
                        sleep(HEALTHCHECK_PROBE_INTERVAL).await;
                    }
                    Err(other_error) => return Err(other_error),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::ReportsHealth;
    use crate::client_health::ClientHealth;
    use crate::error::Error;

    fn unreachable_url() -> Url {
        Url::parse("http://127.0.0.1:1").expect("the test URL must be valid")
    }

    #[tokio::test(start_paused = true)]
    async fn keeps_probing_a_refused_connection() {
        let client_health = ClientHealth::new(unreachable_url());

        assert!(
            timeout(
                Duration::from_secs(1),
                client_health.wait_until_healthy(CancellationToken::new()),
            )
            .await
            .is_err(),
            "a refused connection must keep the probe loop running through every paused-clock retry"
        );
    }

    #[tokio::test]
    async fn a_cancelled_token_stops_the_probe_loop() {
        let client_health = ClientHealth::new(unreachable_url());
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        assert!(matches!(
            client_health.wait_until_healthy(cancellation_token).await,
            Err(Error::RequestCancelled { url }) if url == "http://127.0.0.1:1/health"
        ));
    }
}
