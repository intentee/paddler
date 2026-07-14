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
    use url::Url;

    use super::ClientHealth;
    use crate::error::Error;
    use crate::reports_health::ReportsHealth as _;

    #[tokio::test]
    async fn get_health_reports_an_unreachable_service() {
        let client_health = ClientHealth::new(
            Url::parse("http://127.0.0.1:1").expect("the test URL must be valid"),
        );

        assert!(matches!(
            client_health.get_health().await,
            Err(Error::Connect { .. })
        ));
    }
}
