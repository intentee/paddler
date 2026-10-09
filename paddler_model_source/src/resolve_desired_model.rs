use std::sync::Arc;

use hf_hub::Cache;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_desired_model::AgentDesiredModel;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::huggingface_model_source::HuggingFaceModelSource;
use crate::local_model_path::LocalModelPath;
use crate::model_source::ModelSource;
use crate::model_source_error::ModelSourceError;
use crate::resolves_model_source::ResolvesModelSource;
use crate::url_model_source::UrlModelSource;

pub async fn resolve_desired_model(
    cancellation_token: &CancellationToken,
    desired: &AgentDesiredModel,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
) -> Result<DesiredModelResolution, ModelSourceError> {
    match desired {
        AgentDesiredModel::None => Ok(DesiredModelResolution::NotConfigured),
        AgentDesiredModel::Uri(uri) => match ModelSource::parse(uri, &slot_aggregated_status)? {
            ModelSource::HuggingFace(reference) => {
                HuggingFaceModelSource {
                    cache: Cache::from_env(),
                    reference,
                }
                .resolve(cancellation_token, slot_aggregated_status)
                .await
            }
            ModelSource::LocalToAgent(path) => {
                LocalModelPath::new(path)
                    .resolve(cancellation_token, slot_aggregated_status)
                    .await
            }
            ModelSource::Url(download_url) => {
                UrlModelSource(download_url)
                    .resolve(cancellation_token, slot_aggregated_status)
                    .await
            }
        },
    }
}
