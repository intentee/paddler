use paddler_client::inference_client::InferenceClient;
use paddler_client::inference_client_params::InferenceClientParams;
use url::Url;

#[must_use]
pub fn inference_client_for(url: Url) -> InferenceClient {
    InferenceClient::new(InferenceClientParams {
        socket_pool_size: 1,
        url,
    })
}
