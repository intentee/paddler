use std::sync::Arc;

use anyhow::Result;
use llama_cpp_bindings::context::LlamaContext;
use paddler_agent::build_inference_context_params::build_inference_context_params;
use paddler_agent::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_agent::resolve_inference_thread_count::resolve_inference_thread_count;
use paddler_messaging::inference_parameters::InferenceParameters;

use crate::loaded_test_model::LoadedTestModel;

pub struct EmbeddingProcessorHarness {
    pub llama_context: &'static mut LlamaContext<'static>,
    pub scheduler_context: Arc<ContinuousBatchSchedulerContext>,
}

impl EmbeddingProcessorHarness {
    pub fn build(enable_embeddings: bool) -> Result<Self> {
        Self::build_with_inference_parameters(InferenceParameters {
            enable_embeddings,
            ..InferenceParameters::default()
        })
    }

    pub fn build_with_inference_parameters(
        inference_parameters: InferenceParameters,
    ) -> Result<Self> {
        let loaded = LoadedTestModel::qwen3()?;
        let llama_context: &'static mut LlamaContext<'static> =
            Box::leak(Box::new(loaded.new_context()?));
        let scheduler_context = loaded.scheduler_context(inference_parameters)?;

        Ok(Self {
            llama_context,
            scheduler_context,
        })
    }

    pub fn build_for_embedding_generation(
        inference_parameters: InferenceParameters,
    ) -> Result<Self> {
        let loaded = LoadedTestModel::qwen3()?;
        let scheduler_context = loaded.scheduler_context(inference_parameters.clone())?;
        let context_params = build_inference_context_params(
            &inference_parameters,
            u32::from(scheduler_context.desired_slots_total),
            resolve_inference_thread_count(),
            resolve_inference_thread_count(),
        )?;
        let llama_context: &'static mut LlamaContext<'static> =
            Box::leak(Box::new(loaded.new_context_with_params(context_params)?));

        Ok(Self {
            llama_context,
            scheduler_context,
        })
    }
}
