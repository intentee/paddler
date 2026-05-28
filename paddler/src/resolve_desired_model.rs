use std::sync::Arc;

use anyhow::Result;
use crate::agent_desired_model::AgentDesiredModel;

use crate::desired_model_resolution::DesiredModelResolution;
use crate::model_source::local::LocalModelPath;
use crate::resolves_model_source::ResolvesModelSource;
use crate::slot_aggregated_status::SlotAggregatedStatus;

pub async fn resolve_desired_model(
    desired: &AgentDesiredModel,
    slot_aggregated_status: Arc<SlotAggregatedStatus>,
) -> Result<DesiredModelResolution> {
    match desired {
        AgentDesiredModel::HuggingFace(reference) => {
            reference.resolve(slot_aggregated_status).await
        }
        AgentDesiredModel::LocalToAgent(path) => {
            LocalModelPath::new(path.clone())
                .resolve(slot_aggregated_status)
                .await
        }
        AgentDesiredModel::Url(reference) => reference.resolve(slot_aggregated_status).await,
        AgentDesiredModel::None => Ok(DesiredModelResolution::NotConfigured),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use anyhow::Result;
    use crate::agent_desired_model::AgentDesiredModel;
    use tempfile::NamedTempFile;
    use tempfile::TempDir;

    use crate::desired_model_resolution::DesiredModelResolution;
    use crate::resolve_desired_model::resolve_desired_model;
    use crate::slot_aggregated_status::SlotAggregatedStatus;

    fn fresh_status() -> Arc<SlotAggregatedStatus> {
        Arc::new(SlotAggregatedStatus::new(1))
    }

    fn nonexistent_path_in_temp_dir(label: &str) -> Result<(TempDir, PathBuf)> {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join(format!("missing-{label}.gguf"));

        Ok((dir, path))
    }

    #[tokio::test]
    async fn local_existing_file_resolves_to_resolved_with_that_path() -> Result<()> {
        let status = fresh_status();
        let temp_file = NamedTempFile::new()?;
        let path = temp_file.path().to_path_buf();
        let desired = AgentDesiredModel::LocalToAgent(path.display().to_string());

        let resolution = resolve_desired_model(&desired, status).await?;

        assert!(matches!(
            resolution,
            DesiredModelResolution::Resolved(ref resolved) if *resolved == path
        ));

        Ok(())
    }

    #[tokio::test]
    async fn local_missing_file_resolves_to_local_file_missing_with_that_path() -> Result<()> {
        let status = fresh_status();
        let (_dir_guard, path) = nonexistent_path_in_temp_dir("desired")?;
        let desired = AgentDesiredModel::LocalToAgent(path.display().to_string());

        let resolution = resolve_desired_model(&desired, status).await?;

        assert!(matches!(
            resolution,
            DesiredModelResolution::LocalFileMissing(ref missing) if *missing == path
        ));

        Ok(())
    }

    #[tokio::test]
    async fn none_variant_resolves_to_not_configured() -> Result<()> {
        let status = fresh_status();
        let desired = AgentDesiredModel::None;

        let resolution = resolve_desired_model(&desired, status).await?;

        assert!(matches!(resolution, DesiredModelResolution::NotConfigured));

        Ok(())
    }
}
