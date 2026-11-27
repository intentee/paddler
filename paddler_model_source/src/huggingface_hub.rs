use hf_hub::HFClient;
use hf_hub::HFError;

use crate::huggingface_cache_location::HuggingFaceCacheLocation;
use crate::huggingface_endpoint::HuggingFaceEndpoint;

#[derive(Clone, Debug)]
pub struct HuggingFaceHub {
    pub cache_location: HuggingFaceCacheLocation,
    pub endpoint: HuggingFaceEndpoint,
}

impl HuggingFaceHub {
    /// # Errors
    ///
    /// Returns an [`HFError`] when the endpoint is not a valid URL or the HTTP clients of the Hugging Face client cannot be built.
    pub fn client(&self) -> Result<HFClient, HFError> {
        self.endpoint
            .configured(self.cache_location.client_builder())
            .build()
    }
}
