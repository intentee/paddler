use url::Url;

pub struct InferenceClientParams {
    pub socket_pool_size: usize,
    pub url: Url,
}
