pub mod client_inference;
pub mod client_management;
pub mod error;
mod format_api_url;
mod inference_socket_connection;
mod inference_socket_pool;
mod inference_socket_read_task;
mod inference_socket_url;
mod inference_socket_write_task;
mod stream_ndjson;
mod stream_sse;

pub use client_inference::ClientInference;
pub use client_management::ClientManagement;
pub use error::Error;
pub use error::Result;
use reqwest::Client;
use url::Url;

pub struct PaddlerClient {
    inference_url: Url,
    management_url: Url,
    inference_socket_pool_size: usize,
    http_client: Client,
}

impl PaddlerClient {
    pub fn new(inference_url: Url, management_url: Url, inference_socket_pool_size: usize) -> Self {
        Self {
            inference_url,
            management_url,
            inference_socket_pool_size,
            http_client: Client::new(),
        }
    }

    pub fn inference<'paddler_client>(&'paddler_client self) -> ClientInference<'paddler_client> {
        ClientInference::new(
            &self.inference_url,
            &self.http_client,
            self.inference_socket_pool_size,
        )
    }

    pub fn management<'paddler_client>(&'paddler_client self) -> ClientManagement<'paddler_client> {
        ClientManagement::new(&self.management_url, &self.http_client)
    }
}
