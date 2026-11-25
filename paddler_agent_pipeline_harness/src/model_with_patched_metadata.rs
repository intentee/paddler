use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_gguf::gguf_context::GgufContext;
use llama_cpp_gguf::gguf_context_error::GgufContextError;
use tempfile::TempDir;

use paddler_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_model_card_fixture::model_card::ModelCard;
use paddler_model_source::model_source_error::ModelSourceError;

use crate::write_patched_gguf::write_patched_gguf;

pub struct ModelWithPatchedMetadata {
    model_directory: TempDir,
    source_desired_state: BalancerDesiredState,
}

impl ModelWithPatchedMetadata {
    /// # Errors
    ///
    /// Returns an error when the temporary model directory cannot be created, when the model of the card is not in the Hugging Face cache, and when the model with the patched metadata cannot be written.
    pub fn write<TPatch>(model_card: ModelCard, patch_metadata: TPatch) -> Result<Self>
    where
        TPatch: FnOnce(&mut GgufContext) -> Result<(), GgufContextError>,
    {
        let source_model_path = model_card
            .cached_path()
            .context("the model must be in the Hugging Face cache")?;
        let patched_model = TempDir::new()
            .context("a temporary model directory must be creatable")
            .map(|model_directory| Self {
                model_directory,
                source_desired_state: model_card.into_desired_state(),
            })?;

        write_patched_gguf(
            &source_model_path,
            &patched_model.model_path(),
            patch_metadata,
        )
        .context("the model with patched metadata must be writable")?;

        Ok(patched_model)
    }

    /// # Errors
    ///
    /// Returns [`ModelSourceError::LocalModelPathNotAbsolute`] when the model directory lies at a relative path, and [`ModelSourceError::LocalModelPathClimbsToParentDirectory`] when its path climbs to a parent directory.
    pub fn desired_state(&self) -> Result<BalancerDesiredState, ModelSourceError> {
        agent_desired_model_at(&self.model_path()).map(|model| BalancerDesiredState {
            model,
            ..self.source_desired_state.clone()
        })
    }

    #[must_use]
    pub fn model_path(&self) -> PathBuf {
        self.model_directory
            .path()
            .join("model-with-patched-metadata.gguf")
    }
}
