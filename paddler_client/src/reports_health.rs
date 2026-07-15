use std::future::Future;
use std::time::Duration;

use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

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
                .get_text(cancellation_token, "/health")
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
                    Err(Error::Connect { .. } | Error::ServiceUnavailable { .. }) => {
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
    use std::future::Future;
    use std::sync::Mutex;
    use std::time::Duration;

    use reqwest::StatusCode;
    use tokio::time::timeout;
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::ReportsHealth;
    use crate::client_health::ClientHealth;
    use crate::error::Error;
    use crate::error::Result;
    use crate::http_client::HttpClient;

    const REFUSED_CONNECTION_PROBE_WINDOW: Duration = Duration::from_millis(200);

    fn unreachable_url() -> Url {
        Url::parse("http://127.0.0.1:1").expect("the test URL must be valid")
    }

    fn service_unavailable() -> Error {
        Error::ServiceUnavailable {
            message: "Balancer applicable state is not yet set".to_owned(),
            url: "http://127.0.0.1:1/health".to_owned(),
        }
    }

    fn not_found() -> Error {
        Error::UnexpectedResponseStatus {
            message: "Not Found".to_owned(),
            status: StatusCode::NOT_FOUND,
            url: "http://127.0.0.1:1/health".to_owned(),
        }
    }

    struct ScriptedHealthReporter {
        http_client: HttpClient,
        scripted_probes: Mutex<Vec<Result<String>>>,
    }

    impl ScriptedHealthReporter {
        fn new(scripted_probes: Vec<Result<String>>) -> Self {
            Self {
                http_client: HttpClient::new(unreachable_url()),
                scripted_probes: Mutex::new(scripted_probes.into_iter().rev().collect()),
            }
        }

        fn remaining_probe_count(&self) -> usize {
            self.scripted_probes
                .lock()
                .expect("the probe script must not be poisoned")
                .len()
        }
    }

    impl ReportsHealth for ScriptedHealthReporter {
        fn http_client(&self) -> &HttpClient {
            &self.http_client
        }

        fn get_health(
            &self,
            _cancellation_token: CancellationToken,
        ) -> impl Future<Output = Result<String>> + Send {
            let scripted_probe = self
                .scripted_probes
                .lock()
                .expect("the probe script must not be poisoned")
                .pop()
                .expect("the probe script must not run out of probes");

            async move { scripted_probe }
        }
    }

    #[tokio::test]
    async fn retries_while_the_service_is_unavailable() {
        let reporter = ScriptedHealthReporter::new(vec![
            Err(service_unavailable()),
            Err(service_unavailable()),
            Ok("OK".to_owned()),
        ]);

        reporter
            .wait_until_healthy(CancellationToken::new())
            .await
            .expect("the reporter must become healthy once the service responds");

        assert_eq!(reporter.remaining_probe_count(), 0);
    }

    #[tokio::test]
    async fn propagates_an_unexpected_status_without_retrying() {
        let reporter =
            ScriptedHealthReporter::new(vec![Err(not_found()), Ok("never reached".to_owned())]);

        assert!(matches!(
            reporter.wait_until_healthy(CancellationToken::new()).await,
            Err(Error::UnexpectedResponseStatus { .. })
        ));
        assert_eq!(reporter.remaining_probe_count(), 1);
    }

    #[tokio::test]
    async fn keeps_probing_a_refused_connection() {
        let client_health = ClientHealth::new(unreachable_url());

        assert!(
            timeout(
                REFUSED_CONNECTION_PROBE_WINDOW,
                client_health.wait_until_healthy(CancellationToken::new()),
            )
            .await
            .is_err(),
            "a refused connection must keep the probe loop running"
        );
    }

    #[tokio::test]
    async fn a_cancelled_token_stops_the_probe_loop() {
        let client_health = ClientHealth::new(unreachable_url());
        let cancellation_token = CancellationToken::new();

        cancellation_token.cancel();

        assert!(matches!(
            client_health.wait_until_healthy(cancellation_token).await,
            Err(Error::RequestCancelled { .. })
        ));
    }
}
