use std::sync::Arc;

use llama_cpp_bindings::SampledToken;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::context::params::LlamaContextParams;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::token::LlamaToken;
use tokio_util::sync::CancellationToken;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_messaging::model_metadata::ModelMetadata;
use paddler_spawn_resuming_panic::spawn_blocking_resuming_panic::spawn_blocking_resuming_panic;

use crate::agent_runtime_error::AgentRuntimeError;
use crate::inference_runtime_context::InferenceRuntimeContext;
use crate::inference_thread_count::inference_thread_count;
use crate::llama_context_settings::LlamaContextSettings;
use crate::llama_model_settings::LlamaModelSettings;
use crate::resident_llama_context::ResidentLlamaContext;
use crate::resident_llama_model::ResidentLlamaModel;
use crate::startup_outcome::StartupOutcome;
use crate::vram_occupancy::VramOccupancy;

fn read_model_metadata(model: &LlamaModel) -> Result<ModelMetadata, AgentRuntimeError> {
    let mut model_metadata = ModelMetadata::default();

    for metadata_index in 0..model.meta_count() {
        model_metadata.set_meta_field(
            model
                .meta_key_by_index(metadata_index)
                .map_err(AgentRuntimeError::ModelMetadataUnreadable)?,
            model
                .meta_val_str_by_index(metadata_index)
                .map_err(AgentRuntimeError::ModelMetadataUnreadable)?,
        );
    }

    Ok(model_metadata)
}

#[derive(Clone)]
pub struct LoadedLlamaModel {
    pub inference_thread_count: i32,
    pub llama_model_settings: LlamaModelSettings,
    pub model: Arc<ResidentLlamaModel>,
    pub model_metadata: Arc<ModelMetadata>,
}

impl LoadedLlamaModel {
    pub fn load(
        inference_runtime_context: &InferenceRuntimeContext,
        llama_model_settings: LlamaModelSettings,
    ) -> Result<Self, AgentRuntimeError> {
        let inference_thread_count =
            inference_thread_count(inference_runtime_context.available_parallelism.as_ref())?;
        let vram_occupancy = inference_runtime_context
            .vram_residencies
            .model_weights
            .try_occupy()?;
        let model_issue_path = llama_model_settings.reported_model_path();
        let loaded = Self::load_from_file(
            inference_runtime_context,
            inference_thread_count,
            llama_model_settings,
            vram_occupancy,
        );

        match &loaded {
            Ok(_loaded_llama_model) => {
                inference_runtime_context
                    .slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelIsLoaded(model_issue_path));
            }
            Err(_load_failure) => {
                inference_runtime_context
                    .slot_aggregated_status
                    .register_pipeline_issue(AgentIssue::ModelCannotBeLoaded(model_issue_path));
            }
        }

        loaded
    }

    pub async fn load_unless_shutting_down(
        cancellation_token: &CancellationToken,
        inference_runtime_context: InferenceRuntimeContext,
        llama_model_settings: LlamaModelSettings,
    ) -> Result<StartupOutcome<Self>, AgentRuntimeError> {
        if cancellation_token.is_cancelled() {
            return Ok(StartupOutcome::Cancelled);
        }

        let loaded_llama_model = spawn_blocking_resuming_panic(move || {
            Self::load(&inference_runtime_context, llama_model_settings)
        })
        .await?;

        if cancellation_token.is_cancelled() {
            loaded_llama_model.release().await;

            return Ok(StartupOutcome::Cancelled);
        }

        Ok(StartupOutcome::Ready(loaded_llama_model))
    }

    pub async fn release(self) {
        spawn_blocking_resuming_panic(move || drop(self)).await;
    }

    #[must_use]
    pub const fn llama_context_settings<'parameters>(
        &self,
        model_runtime_parameters: &'parameters ModelRuntimeParameters,
        n_seq_max: u32,
    ) -> LlamaContextSettings<'parameters> {
        LlamaContextSettings {
            model_runtime_parameters,
            n_seq_max,
            thread_count: self.inference_thread_count,
        }
    }

    pub fn create_llama_context(
        &self,
        inference_runtime_context: &InferenceRuntimeContext,
        inference_capability: InferenceCapability,
        llama_context_params: LlamaContextParams,
    ) -> Result<ResidentLlamaContext<'_>, AgentRuntimeError> {
        let vram_occupancy = inference_runtime_context
            .vram_residencies
            .llama_context(inference_capability)
            .try_occupy()?;
        let n_seq_max = llama_context_params.n_seq_max();

        #[expect(
            clippy::disallowed_methods,
            reason = "the context is created here, paired with the VRAM occupancy of its capability"
        )]
        let created_llama_context = LlamaContext::from_model(
            &self.model.llama_model,
            &self.model.llama_backend,
            llama_context_params,
        );

        created_llama_context
            .map(|llama_context| ResidentLlamaContext {
                llama_context,
                vram_occupancy,
            })
            .map_err(|source| {
                let context_creation_failure = AgentRuntimeError::ContextCreationFailed(source);
                let error = context_creation_failure.to_string();

                for slot_index in 0..n_seq_max {
                    inference_runtime_context
                        .slot_aggregated_status
                        .register_pipeline_issue(AgentIssue::SlotCannotStart(
                            SlotCannotStartParams {
                                error: error.clone(),
                                inference_capability,
                                slot_index,
                            },
                        ));
                }

                context_creation_failure
            })
    }

    pub fn warm_up_llama_context(
        &self,
        llama_context: &mut LlamaContext<'_>,
        warmup_batch: &mut LlamaBatch<'static>,
        desired_slots_total: u16,
    ) -> Result<(), AgentRuntimeError> {
        let warmup_tokens: Vec<LlamaToken> = [self.model.token_bos(), self.model.token_eos()]
            .into_iter()
            .flatten()
            .collect();

        llama_context.clear_kv_cache();

        let warm_up = Self::decode_every_warm_up_position(
            llama_context,
            warmup_batch,
            &warmup_tokens,
            desired_slots_total,
        );

        llama_context.synchronize();
        llama_context.clear_kv_cache();

        warm_up
    }

    pub fn warm_up_and_serve<'model, TServe>(
        &self,
        mut llama_context: ResidentLlamaContext<'model>,
        mut warmup_batch: LlamaBatch<'static>,
        desired_slots_total: u16,
        serve: TServe,
    ) -> Result<(), AgentRuntimeError>
    where
        TServe: FnOnce(ResidentLlamaContext<'model>, LlamaBatch<'static>),
    {
        self.warm_up_llama_context(&mut llama_context, &mut warmup_batch, desired_slots_total)?;

        serve(llama_context, warmup_batch);

        Ok(())
    }

    fn decode_every_warm_up_position(
        llama_context: &mut LlamaContext<'_>,
        warmup_batch: &mut LlamaBatch<'static>,
        warmup_tokens: &[LlamaToken],
        desired_slots_total: u16,
    ) -> Result<(), AgentRuntimeError> {
        for (position, warmup_token) in (0..).zip(warmup_tokens) {
            warmup_batch.clear();

            for sequence_index in 0..i32::from(desired_slots_total) {
                warmup_batch
                    .add(
                        &SampledToken::Content(*warmup_token),
                        position,
                        &[sequence_index],
                        true,
                    )
                    .map_err(AgentRuntimeError::WarmUpBatchAssemblyFailed)?;
            }

            llama_context
                .decode(warmup_batch)
                .map_err(AgentRuntimeError::WarmUpDecodeFailed)?;
        }

        Ok(())
    }

    fn load_from_file(
        inference_runtime_context: &InferenceRuntimeContext,
        inference_thread_count: i32,
        llama_model_settings: LlamaModelSettings,
        vram_occupancy: VramOccupancy,
    ) -> Result<Self, AgentRuntimeError> {
        let llama_backend = inference_runtime_context
            .llama_backend
            .get_or_init()
            .map_err(AgentRuntimeError::BackendInitializationFailed)?;

        #[expect(
            clippy::disallowed_methods,
            reason = "the model is loaded here, paired with the VRAM occupancy of the model weights"
        )]
        let loaded_model = LlamaModel::load_from_file(
            &llama_backend,
            &llama_model_settings.model_path,
            &llama_model_settings.to_llama_model_params(),
        );

        let model = loaded_model.map_err(|source| AgentRuntimeError::ModelLoadFailed {
            model_path: llama_model_settings.model_path.clone(),
            source,
        })?;
        let model_metadata = read_model_metadata(&model)?;

        Ok(Self {
            inference_thread_count,
            llama_model_settings,
            model: Arc::new(ResidentLlamaModel {
                llama_model: model,
                llama_backend,
                vram_occupancy,
            }),
            model_metadata: Arc::new(model_metadata),
        })
    }
}
