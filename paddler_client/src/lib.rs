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

pub use agents_stream::AgentsStream;
pub use buffered_requests_stream::BufferedRequestsStream;
pub use client_inference::ClientInference;
pub use client_management::ClientManagement;
pub use error::Error;
pub use error::Result;
pub use inference_message_stream::InferenceMessageStream;

pub struct PaddlerClient {
    inference_url: Url,
    management_url: Url,
    inference_socket_pool_size: usize,
    http_client: Client,
}

impl PaddlerClient {
    #[must_use]
    pub fn new(inference_url: Url, management_url: Url, inference_socket_pool_size: usize) -> Self {
        Self {
            inference_url,
            management_url,
            inference_socket_pool_size,
            http_client: Client::new(),
        }
    }

    #[must_use]
    pub const fn inference(&self) -> ClientInference<'_> {
        ClientInference::new(
            &self.inference_url,
            &self.http_client,
            self.inference_socket_pool_size,
        )
    }

    #[must_use]
    pub const fn management(&self) -> ClientManagement<'_> {
        ClientManagement::new(&self.management_url, &self.http_client)
    }
}
