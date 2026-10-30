use std::path::PathBuf;

use hf_hub::HFClient;
use hf_hub::HFClientBuilder;

#[derive(Clone, Debug)]
pub enum HuggingFaceCacheLocation {
    Directory(PathBuf),
    HubDefault,
}

impl HuggingFaceCacheLocation {
    #[must_use]
    pub fn client_builder(&self) -> HFClientBuilder {
        match self {
            Self::Directory(cache_dir) => HFClient::builder().cache_dir(cache_dir.clone()),
            Self::HubDefault => HFClient::builder(),
        }
    }
}
