use std::sync::Arc;

use hf_hub::HFClient;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_cache_dir::cache_dir::CacheDir;
use paddler_download_manager::download_manager::DownloadManager;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::huggingface_cache_location::HuggingFaceCacheLocation;
use crate::huggingface_hub::HuggingFaceHub;
use crate::huggingface_model_source::HuggingFaceModelSource;
use crate::local_model_path::LocalModelPath;
use crate::model_source::ModelSource;
use crate::model_source_error::ModelSourceError;
use crate::resolves_model_source::ResolvesModelSource as _;
use crate::url_model_source::UrlModelSource;

#[derive(Clone)]
pub struct DesiredModelResolver {
    pub cache_dir: CacheDir,
    pub download_manager: DownloadManager,
    pub huggingface_client: HFClient,
}

impl DesiredModelResolver {
    pub fn from_process_env(huggingface_endpoint: String) -> Result<Self, ModelSourceError> {
        Ok(Self {
            cache_dir: CacheDir::from_process_env(),
            download_manager: DownloadManager::new()
                .map_err(ModelSourceError::DownloadClientUnavailable)?,
            huggingface_client: HuggingFaceHub {
                cache_location: HuggingFaceCacheLocation::HubDefault,
                endpoint: huggingface_endpoint,
            }
            .client()
            .map_err(ModelSourceError::HuggingFaceClientUnavailable)?,
        })
    }

    pub async fn resolve_uri(
        &self,
        cancellation_token: &CancellationToken,
        uri: &str,
        slot_aggregated_status: Arc<SlotAggregatedStatus>,
    ) -> Result<DesiredModelResolution, ModelSourceError> {
        match ModelSource::parse(uri, &slot_aggregated_status)? {
            ModelSource::HuggingFace(reference) => {
                HuggingFaceModelSource {
                    client: self.huggingface_client.clone(),
                    reference,
                }
                .resolve(cancellation_token, slot_aggregated_status)
                .await
            }
            ModelSource::LocalToAgent(agent_local_model_path) => {
                LocalModelPath {
                    path: agent_local_model_path.path().to_path_buf(),
                }
                .resolve(cancellation_token, slot_aggregated_status)
                .await
            }
            ModelSource::Url(download_url) => {
                UrlModelSource {
                    cache_dir: self.cache_dir.clone(),
                    download_manager: self.download_manager.clone(),
                    download_url,
                }
                .resolve(cancellation_token, slot_aggregated_status)
                .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::sync::Arc;

    use hf_hub::HFError;
    use tempfile::tempdir;
    use tokio_util::sync::CancellationToken;
    use url::ParseError;
    use url::Url;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;

    use super::DesiredModelResolver;
    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::huggingface_default_endpoint::HUGGINGFACE_DEFAULT_ENDPOINT;
    use crate::model_source_error::ModelSourceError;

    #[test]
    fn a_malformed_hugging_face_endpoint_keeps_the_resolver_from_being_built() {
        assert_eq!(
            DesiredModelResolver::from_process_env("not an endpoint".to_owned())
                .err()
                .map(|construction_error| construction_error.to_string()),
            Some(
                ModelSourceError::HuggingFaceClientUnavailable(HFError::Url(
                    ParseError::RelativeUrlWithoutBase
                ))
                .to_string()
            )
        );
    }

    #[tokio::test]
    async fn resolves_an_agent_uri_whose_file_name_is_percent_encoded() {
        let model_directory = tempdir().unwrap();
        let model_path = model_directory.path().join("my modèl.gguf");

        File::create(&model_path).unwrap();

        let resolution =
            DesiredModelResolver::from_process_env(HUGGINGFACE_DEFAULT_ENDPOINT.to_owned())
                .unwrap()
                .resolve_uri(
                    &CancellationToken::new(),
                    &format!(
                        "agent://{}/my%20mod%C3%A8l.gguf",
                        Url::from_file_path(model_directory.path()).unwrap().path()
                    ),
                    Arc::new(SlotAggregatedStatus::new(1)),
                )
                .await
                .unwrap();

        assert_eq!(resolution, DesiredModelResolution::Resolved(model_path));
    }
}
