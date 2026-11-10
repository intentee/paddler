use tokio_util::sync::CancellationToken;

use paddler_spawn_resuming_panic::spawn_blocking_resuming_panic::spawn_blocking_resuming_panic;

use crate::agent_runtime_error::AgentRuntimeError;
use crate::inference_runtime_context::InferenceRuntimeContext;
use crate::llama_model_settings::LlamaModelSettings;
use crate::loaded_llama_model::LoadedLlamaModel;
use crate::startup_outcome::StartupOutcome;

pub struct LlamaModelLoader {
    pub inference_runtime_context: InferenceRuntimeContext,
    pub llama_model_settings: LlamaModelSettings,
}

impl LlamaModelLoader {
    /// # Errors
    ///
    /// Returns the [`AgentRuntimeError`] of [`LoadedLlamaModel::load`] when the model cannot be loaded.
    pub async fn load_unless_shutting_down(
        self,
        cancellation_token: &CancellationToken,
    ) -> Result<StartupOutcome<LoadedLlamaModel>, AgentRuntimeError> {
        if cancellation_token.is_cancelled() {
            return Ok(StartupOutcome::Cancelled);
        }

        let Self {
            inference_runtime_context,
            llama_model_settings,
        } = self;
        let loaded_llama_model = spawn_blocking_resuming_panic(move || {
            LoadedLlamaModel::load(&inference_runtime_context, llama_model_settings)
        })
        .await?;

        if cancellation_token.is_cancelled() {
            loaded_llama_model.release().await;

            return Ok(StartupOutcome::Cancelled);
        }

        Ok(StartupOutcome::Ready(loaded_llama_model))
    }
}
