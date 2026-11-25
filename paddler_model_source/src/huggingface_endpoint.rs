use hf_hub::HFClientBuilder;

#[derive(Clone, Debug)]
pub enum HuggingFaceEndpoint {
    Explicit(String),
    HubDefault,
}

impl HuggingFaceEndpoint {
    #[must_use]
    pub fn configured(&self, client_builder: HFClientBuilder) -> HFClientBuilder {
        match self {
            Self::Explicit(endpoint) => client_builder.endpoint(endpoint.clone()),
            Self::HubDefault => client_builder,
        }
    }
}
