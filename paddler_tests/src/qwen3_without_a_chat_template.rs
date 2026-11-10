use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use tempfile::TempDir;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::constants::EMBEDDED_CHAT_TEMPLATE_KEY;
use paddler_model_source::model_source_error::ModelSourceError;
use paddler_test_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;

use crate::qwen3_desired_state::qwen3_desired_state;
use crate::write_patched_gguf::write_patched_gguf;

pub struct Qwen3WithoutAChatTemplate {
    model_directory: TempDir,
}

impl Qwen3WithoutAChatTemplate {
    /// # Errors
    ///
    /// Returns an error when the temporary model directory cannot be created, when Qwen3 is not in the Hugging Face cache, and when the model without its chat template cannot be written.
    pub fn write() -> Result<Self> {
        TempDir::new()
            .context("a temporary model directory must be creatable")
            .map(|model_directory| Self { model_directory })
            .and_then(Self::with_the_chat_template_removed)
    }

    fn with_the_chat_template_removed(self) -> Result<Self> {
        qwen3_0_6b()
            .cached_path()
            .context("Qwen3 must be in the Hugging Face cache")
            .and_then(|qwen3_path| {
                write_patched_gguf(&qwen3_path, &self.model_path(), |gguf_context| {
                    gguf_context.remove_key(EMBEDDED_CHAT_TEMPLATE_KEY)
                })
                .context("the model without a chat template must be writable")
            })
            .map(|()| self)
    }

    /// # Errors
    ///
    /// Returns [`ModelSourceError::LocalModelPathNotAbsolute`] when the model directory lies at a relative path, and [`ModelSourceError::LocalModelPathClimbsToParentDirectory`] when its path climbs to a parent directory.
    pub fn desired_state(&self) -> Result<BalancerDesiredState, ModelSourceError> {
        agent_desired_model_at(&self.model_path()).map(|model| BalancerDesiredState {
            model,
            ..qwen3_desired_state()
        })
    }

    #[must_use]
    pub fn model_path(&self) -> PathBuf {
        self.model_directory
            .path()
            .join("qwen3-without-a-chat-template.gguf")
    }
}
