use std::sync::Arc;

use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::context::context_limits::ContextLimits;
use llama_cpp_bindings::context::params::LlamaContextParams;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::sampled_token::SampledToken;
use llama_cpp_bindings::token::LlamaToken;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
use paddler_messaging::inference_capability::InferenceCapability;
use paddler_messaging::model_metadata::ModelMetadata;
use paddler_spawn_resuming_panic::spawn_blocking_resuming_panic::spawn_blocking_resuming_panic;

use crate::agent_runtime_error::AgentRuntimeError;
use crate::inference_parallelism::InferenceParallelism;
use crate::inference_runtime_context::InferenceRuntimeContext;
use crate::llama_context_settings::LlamaContextSettings;
use crate::llama_model_settings::LlamaModelSettings;
use crate::model_file_prefetch::ModelFilePrefetch;
use crate::resident_llama_context::ResidentLlamaContext;
use crate::resident_llama_model::ResidentLlamaModel;
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
    pub inference_parallelism: InferenceParallelism,
    pub llama_model_settings: LlamaModelSettings,
    pub model: Arc<ResidentLlamaModel>,
    pub model_metadata: Arc<ModelMetadata>,
}

impl LoadedLlamaModel {
    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::AvailableParallelismUnknown`] or [`AgentRuntimeError::ThreadCountOutOfRange`] when the inference thread count cannot be determined, [`AgentRuntimeError::ModelWeightsAlreadyResident`] when another model occupies the VRAM, [`AgentRuntimeError::BackendInitializationFailed`] when the llama.cpp backend cannot start, [`AgentRuntimeError::ModelFilePrefetchFailed`] when the model file cannot be read, [`AgentRuntimeError::ModelLoadFailed`] when llama.cpp cannot load the model file, and [`AgentRuntimeError::ModelMetadataUnreadable`] when its metadata cannot be read.
    pub fn load(
        inference_runtime_context: &InferenceRuntimeContext,
        llama_model_settings: LlamaModelSettings,
    ) -> Result<Self, AgentRuntimeError> {
        let inference_parallelism = inference_runtime_context
            .available_parallelism
            .available_parallelism()
            .map_err(AgentRuntimeError::AvailableParallelismUnknown)
            .and_then(InferenceParallelism::of_available_parallelism)?;
        let vram_occupancy = inference_runtime_context
            .vram_residencies
            .model_weights
            .try_occupy()?;
        let model_issue_path = llama_model_settings.reported_model_path();
        let loaded = Self::load_from_file(
            inference_runtime_context,
            inference_parallelism,
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

    fn load_from_file(
        InferenceRuntimeContext { llama_backend, .. }: &InferenceRuntimeContext,
        inference_parallelism: InferenceParallelism,
        llama_model_settings: LlamaModelSettings,
        vram_occupancy: VramOccupancy,
    ) -> Result<Self, AgentRuntimeError> {
        let llama_backend = llama_backend
            .get_or_init()
            .map_err(AgentRuntimeError::BackendInitializationFailed)?;

        let model = ModelFilePrefetch {
            model_path: &llama_model_settings.model_path,
            stream_count: inference_parallelism.available_parallelism,
        }
        .alongside(|| {
            LlamaModel::load_from_file(
                &llama_backend,
                &llama_model_settings.model_path,
                &llama_model_settings.to_llama_model_params(),
            )
            .map_err(|source| AgentRuntimeError::ModelLoadFailed {
                model_path: llama_model_settings.model_path.clone(),
                source,
            })
        })?;
        let model_metadata = read_model_metadata(&model)?;

        Ok(Self {
            inference_parallelism,
            llama_model_settings,
            model: Arc::new(ResidentLlamaModel {
                llama_model: model,
                llama_backend,
                vram_occupancy,
            }),
            model_metadata: Arc::new(model_metadata),
        })
    }

    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::LlamaContextAlreadyResident`] when a context of the same capability occupies the VRAM, and [`AgentRuntimeError::ContextCreationFailed`] when llama.cpp cannot create the context.
    pub fn create_llama_context(
        &self,
        InferenceRuntimeContext {
            slot_aggregated_status,
            vram_residencies,
            ..
        }: &InferenceRuntimeContext,
        inference_capability: InferenceCapability,
        llama_context_params: LlamaContextParams,
    ) -> Result<ResidentLlamaContext<'_>, AgentRuntimeError> {
        let vram_occupancy = vram_residencies
            .llama_context(inference_capability)
            .try_occupy()?;
        let n_seq_max = llama_context_params.n_seq_max();

        LlamaContext::from_model(
            &self.model.llama_model,
            &self.model.llama_backend,
            llama_context_params,
        )
        .map(|llama_context| ResidentLlamaContext {
            llama_context,
            vram_occupancy,
        })
        .map_err(|source| {
            let context_creation_failure = AgentRuntimeError::ContextCreationFailed(source);
            let error = context_creation_failure.to_string();

            for slot_index in 0..n_seq_max {
                slot_aggregated_status.register_pipeline_issue(AgentIssue::SlotCannotStart(
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

    #[must_use]
    pub const fn llama_context_settings<'parameters>(
        &self,
        model_runtime_parameters: &'parameters ModelRuntimeParameters,
        n_seq_max: u32,
    ) -> LlamaContextSettings<'parameters> {
        LlamaContextSettings {
            model_runtime_parameters,
            n_seq_max,
            thread_count: self.inference_parallelism.inference_thread_count,
        }
    }

    pub async fn release(self) {
        spawn_blocking_resuming_panic(move || drop(self)).await;
    }

    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::WarmUpBatchAssemblyFailed`] when the warm-up tokens do not fit the batch, and [`AgentRuntimeError::WarmUpDecodeFailed`] when llama.cpp rejects the warm-up batch, without serving the context.
    pub fn warm_up_and_serve<'model, TServe>(
        &self,
        mut llama_context: ResidentLlamaContext<'model>,
        serve: TServe,
    ) -> Result<(), AgentRuntimeError>
    where
        TServe: FnOnce(ResidentLlamaContext<'model>, LlamaBatch),
    {
        let mut batch = LlamaBatch::new(ContextLimits::of(&llama_context));

        self.warm_up_llama_context(&mut llama_context, &mut batch)?;

        serve(llama_context, batch);

        Ok(())
    }

    /// # Errors
    ///
    /// Returns [`AgentRuntimeError::WarmUpBatchAssemblyFailed`] when the warm-up tokens do not fit the batch, and [`AgentRuntimeError::WarmUpDecodeFailed`] when llama.cpp rejects the warm-up batch.
    pub fn warm_up_llama_context(
        &self,
        llama_context: &mut LlamaContext<'_>,
        warmup_batch: &mut LlamaBatch,
    ) -> Result<(), AgentRuntimeError> {
        let warmup_tokens: Vec<LlamaToken> = [self.model.token_bos(), self.model.token_eos()]
            .into_iter()
            .flatten()
            .collect();

        llama_context.memory_clear();

        let warm_up =
            Self::decode_every_warm_up_position(llama_context, warmup_batch, &warmup_tokens);

        llama_context.memory_clear();

        warm_up
    }

    fn decode_every_warm_up_position(
        llama_context: &mut LlamaContext<'_>,
        warmup_batch: &mut LlamaBatch,
        warmup_tokens: &[LlamaToken],
    ) -> Result<(), AgentRuntimeError> {
        let warmed_up_sequences = warmup_batch.limits().n_seq_max.cast_signed();

        for (position, warmup_token) in (0..).zip(warmup_tokens) {
            warmup_batch.clear();

            for sequence_id in 0..warmed_up_sequences {
                warmup_batch
                    .add(
                        &SampledToken::Content(*warmup_token),
                        position,
                        &[sequence_id],
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
}
