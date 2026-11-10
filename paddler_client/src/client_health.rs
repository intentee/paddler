use url::Url;

use crate::http_client::HttpClient;
use crate::reports_health::ReportsHealth;

#[derive(Clone)]
pub struct ClientHealth {
    http_client: HttpClient,
}

impl ClientHealth {
    #[must_use]
    pub fn new(url: Url) -> Self {
        Self {
            http_client: HttpClient::new(url),
        }
    }
}

impl ReportsHealth for ClientHealth {
    fn http_client(&self) -> &HttpClient {
        &self.http_client
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::time::timeout;
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::ClientHealth;
    use crate::error::Error;
    use crate::reports_health::ReportsHealth as _;

    fn unreachable_url() -> Url {
        Url::parse("http://127.0.0.1:1").expect("the test URL must be valid")
    }

    #[tokio::test]
    async fn get_health_reports_an_unreachable_service() {
        let client_health = ClientHealth::new(
            Url::parse("http://127.0.0.1:1").expect("the test URL must be valid"),
        );

        assert!(matches!(
            client_health.get_health(CancellationToken::new()).await,
            Err(Error::Connect { url, .. }) if url == "http://127.0.0.1:1/health"
        ));
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
