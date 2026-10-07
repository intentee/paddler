use std::path::Path;
use std::sync::Arc;

use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::context::params::LlamaContextParams;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;
use log::warn;

use paddler_agent_status::agent_issue_fix::AgentIssueFix;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_issue_params::slot_cannot_start_params::SlotCannotStartParams;
use paddler_messaging::model_metadata::ModelMetadata;

use crate::agent_runtime_error::AgentRuntimeError;
use crate::describe_error_with_causes::describe_error_with_causes;
use crate::inference_runtime_context::InferenceRuntimeContext;

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
    pub llama_backend: Arc<LlamaBackend>,
    pub model: Arc<LlamaModel>,
}

impl LoadedLlamaModel {
    pub fn load(
        inference_runtime_context: &InferenceRuntimeContext,
        model_path: &Path,
        model_params: &LlamaModelParams,
    ) -> Result<Self, AgentRuntimeError> {
        let model_issue_path = ModelPath::from(model_path);
        let loaded = Self::load_from_file(model_path, model_params);

        match &loaded {
            Ok(Self { model, .. }) => {
                inference_runtime_context
                    .slot_aggregated_status
                    .register_fix(&AgentIssueFix::ModelIsLoaded(model_issue_path));
                inference_runtime_context
                    .model_metadata_holder
                    .set_model_metadata(read_model_metadata(model)?);
            }
            Err(_load_failure) => {
                inference_runtime_context
                    .slot_aggregated_status
                    .register_issue(AgentIssue::ModelCannotBeLoaded(model_issue_path));
            }
        }

        loaded
    }

    pub fn create_llama_context(
        &self,
        inference_runtime_context: &InferenceRuntimeContext,
        llama_context_params: LlamaContextParams,
    ) -> Result<LlamaContext<'_>, AgentRuntimeError> {
        let n_seq_max = llama_context_params.n_seq_max();

        LlamaContext::from_model(&self.model, &self.llama_backend, llama_context_params).map_err(
            |source| {
                let context_creation_failure = AgentRuntimeError::ContextCreationFailed(source);
                let error = describe_error_with_causes(&context_creation_failure);

                for slot_index in 0..n_seq_max {
                    inference_runtime_context
                        .slot_aggregated_status
                        .register_issue(AgentIssue::SlotCannotStart(SlotCannotStartParams {
                            error: error.clone(),
                            slot_index,
                        }));
                }

                context_creation_failure
            },
        )
    }

    pub fn warm_up_llama_context(
        &self,
        llama_context: &mut LlamaContext<'_>,
        warmup_batch: &mut LlamaBatch<'static>,
        desired_slots_total: u16,
    ) {
        let warmup_tokens = [self.model.token_bos(), self.model.token_eos()];

        for sequence_index in 0..i32::from(desired_slots_total) {
            if let Err(batch_add_error) =
                warmup_batch.add_sequence(&warmup_tokens, sequence_index, true)
            {
                warn!("Warmup batch add_sequence failed: {batch_add_error:#}");
                return;
            }
        }

        llama_context.clear_kv_cache();
        if let Err(decode_error) = llama_context.decode(warmup_batch) {
            warn!("Warmup decode failed: {decode_error:#}");
        }
        llama_context.synchronize();
        llama_context.clear_kv_cache();
    }

    fn load_from_file(
        model_path: &Path,
        model_params: &LlamaModelParams,
    ) -> Result<Self, AgentRuntimeError> {
        let llama_backend =
            Arc::new(LlamaBackend::init().map_err(AgentRuntimeError::BackendInitializationFailed)?);
        let model = LlamaModel::load_from_file(&llama_backend, model_path, model_params).map_err(
            |source| AgentRuntimeError::ModelLoadFailed {
                model_path: model_path.to_path_buf(),
                source,
            },
        )?;

        Ok(Self {
            llama_backend,
            model: Arc::new(model),
        })
    }
}
