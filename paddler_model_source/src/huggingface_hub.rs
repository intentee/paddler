use hf_hub::HFClient;
use hf_hub::HFError;

use crate::huggingface_cache_location::HuggingFaceCacheLocation;

#[derive(Clone, Debug)]
pub struct HuggingFaceHub {
    pub cache_location: HuggingFaceCacheLocation,
    pub endpoint: String,
}

impl HuggingFaceHub {
    pub fn client(&self) -> Result<HFClient, HFError> {
        self.cache_location
            .client_builder()
            .endpoint(self.endpoint.clone())
            .build()
    }
}
