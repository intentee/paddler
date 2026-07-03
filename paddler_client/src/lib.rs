pub mod agents_stream;
pub mod buffered_requests_stream;
pub mod client_inference;
pub mod client_management;
pub mod error;
mod format_api_url;
pub mod inference_message_stream;
mod inference_socket;
mod stream;

use reqwest::Client;
use url::Url;

use crate::client_inference::ClientInference;
use crate::client_management::ClientManagement;
use crate::inference_socket::pool::Pool;

pub struct PaddlerClient {
    inference_url: Url,
    management_url: Url,
    http_client: Client,
    inference_socket_pool: Pool,
}

impl PaddlerClient {
    #[must_use]
    pub fn new(inference_url: Url, management_url: Url, inference_socket_pool_size: usize) -> Self {
        let inference_socket_pool = Pool::new(inference_url.clone(), inference_socket_pool_size);

        Self {
            inference_url,
            management_url,
            http_client: Client::new(),
            inference_socket_pool,
        }
    }

    #[must_use]
    pub const fn inference(&self) -> ClientInference<'_> {
        ClientInference::new(
            &self.inference_url,
            &self.http_client,
            &self.inference_socket_pool,
        )
    }

    #[must_use]
    pub const fn management(&self) -> ClientManagement<'_> {
        ClientManagement::new(&self.management_url, &self.http_client)
    }
}
