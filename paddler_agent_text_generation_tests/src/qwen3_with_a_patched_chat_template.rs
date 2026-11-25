use std::ffi::CString;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_gguf::gguf_context::GgufContext;
use llama_cpp_gguf::gguf_context_error::GgufContextError;
use tempfile::TempDir;

use paddler_agent_pipeline_harness::write_patched_gguf::write_patched_gguf;
use paddler_balancer_desired_state_fixture::qwen3_desired_state::qwen3_desired_state;
use paddler_cluster_harness::agent_desired_model_at::agent_desired_model_at;
use paddler_messaging::balancer_desired_state::BalancerDesiredState;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;
use paddler_model_source::model_source_error::ModelSourceError;

const CHAT_TEMPLATE_GGUF_KEY: &str = "tokenizer.chat_template";

pub struct Qwen3WithAPatchedChatTemplate {
    model_directory: TempDir,
}

impl Qwen3WithAPatchedChatTemplate {
    /// # Errors
    ///
    /// Returns an error when the chat template holds a NUL byte, when the temporary model directory cannot be created, when Qwen3 is not in the Hugging Face cache, and when the model with the embedded chat template cannot be written.
    pub fn with_an_embedded_chat_template(ChatTemplate { content }: &ChatTemplate) -> Result<Self> {
        CString::new(CHAT_TEMPLATE_GGUF_KEY)
            .context("the chat template key must hold no NUL byte")
            .and_then(|chat_template_key| {
                CString::new(content.as_str())
                    .context("the chat template must hold no NUL byte")
                    .and_then(|chat_template_content| {
                        Self::written(|gguf_context| {
                            gguf_context.set_val_str(&chat_template_key, &chat_template_content);

                            Ok(())
                        })
                    })
            })
    }

    /// # Errors
    ///
    /// Returns an error when the temporary model directory cannot be created, when Qwen3 is not in the Hugging Face cache, and when the model without its chat template cannot be written.
    pub fn without_a_chat_template() -> Result<Self> {
        Self::written(|gguf_context| gguf_context.remove_key(CHAT_TEMPLATE_GGUF_KEY))
    }

    fn with_the_patch_applied<TPatch>(self, patch: TPatch) -> Result<Self>
    where
        TPatch: FnOnce(&mut GgufContext) -> Result<(), GgufContextError>,
    {
        qwen3_0_6b()
            .cached_path()
            .context("Qwen3 must be in the Hugging Face cache")
            .and_then(|qwen3_path| {
                write_patched_gguf(&qwen3_path, &self.model_path(), patch)
                    .context("the patched model must be writable")
            })
            .map(|()| self)
    }

    fn written<TPatch>(patch: TPatch) -> Result<Self>
    where
        TPatch: FnOnce(&mut GgufContext) -> Result<(), GgufContextError>,
    {
        TempDir::new()
            .context("a temporary model directory must be creatable")
            .map(|model_directory| Self { model_directory })
            .and_then(|patched_model| patched_model.with_the_patch_applied(patch))
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
            .join("qwen3-with-a-patched-chat-template.gguf")
    }
}
