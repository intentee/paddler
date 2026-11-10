use std::future::Future;
use std::time::Duration;

use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

use paddler_messaging::constants::API_PATH_HEALTH;

use crate::error::Error;
use crate::http_client::HttpClient;
use crate::result::Result;

const HEALTHCHECK_PROBE_INTERVAL: Duration = Duration::from_millis(20);

pub trait ReportsHealth: Sync {
    fn get_health(
        &self,
        cancellation_token: CancellationToken,
    ) -> impl Future<Output = Result<String>> + Send {
        async move {
            self.http_client()
                .get_text(cancellation_token, API_PATH_HEALTH)
                .await
        }
    }

    fn http_client(&self) -> &HttpClient;

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
