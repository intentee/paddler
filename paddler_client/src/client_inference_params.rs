use std::num::NonZeroUsize;

use url::Url;

pub struct ClientInferenceParams {
    pub inference_socket_pool_size: NonZeroUsize,
    pub url: Url,
}
