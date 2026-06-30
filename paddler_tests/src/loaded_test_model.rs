use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use llama_cpp_bindings::SampledTokenClassifier;
use llama_cpp_bindings::context::LlamaContext;
use llama_cpp_bindings::context::params::LlamaContextParams;
use llama_cpp_bindings::llama_backend::LlamaBackend;
use llama_cpp_bindings::llama_batch::LlamaBatch;
use llama_cpp_bindings::model::LlamaModel;
use llama_cpp_bindings::model::params::LlamaModelParams;
use llama_cpp_bindings::mtmd::MtmdContext;
use llama_cpp_bindings::mtmd::MtmdContextParams;
use paddler_agent::chat_template_renderer::ChatTemplateRenderer;
use paddler_agent::continuous_batch_scheduler_context::ContinuousBatchSchedulerContext;
use paddler_messaging::chat_template::ChatTemplate;
use paddler_messaging::inference_parameters::InferenceParameters;
use paddler_model_card::model_card::ModelCard;
use paddler_model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_model_card::smolvlm2_256m_mmproj::smolvlm2_256m_mmproj;

fn resolve_model_card_file(model_card: &ModelCard) -> Result<PathBuf> {
    Ok(hf_hub::api::sync::ApiBuilder::from_env()
        .build()?
        .model(model_card.reference.repo_id.clone())
        .get(&model_card.reference.filename)?)
}

pub struct LoadedTestModel {
    backend: &'static LlamaBackend,
    model: Arc<LlamaModel>,
    model_static: &'static LlamaModel,
    model_path: PathBuf,
}

impl LoadedTestModel {
    pub fn qwen3() -> Result<Self> {
        Self::from_model_card(qwen3_0_6b())
    }

    pub fn smolvlm2() -> Result<Self> {
        Self::from_model_card(smolvlm2_256m())
    }

    fn from_model_card(model_card: ModelCard) -> Result<Self> {
        let model_path = resolve_model_card_file(&model_card)?;

        let backend: &'static LlamaBackend = Box::leak(Box::new(LlamaBackend::init()?));
        let model_params =
            LlamaModelParams::default().with_n_gpu_layers(model_card.gpu_layer_count);
        let model = Arc::new(LlamaModel::load_from_file(
            backend,
            model_path.clone(),
            &model_params,
        )?);
        let model_static: &'static LlamaModel = Box::leak(Box::new(Arc::clone(&model)));

        Ok(Self {
            backend,
            model,
            model_static,
            model_path,
        })
    }

    pub fn new_context(&self) -> Result<LlamaContext<'static>> {
        self.new_context_with_params(LlamaContextParams::default())
    }

    pub fn new_context_with_params(
        &self,
        context_params: LlamaContextParams,
    ) -> Result<LlamaContext<'static>> {
        Ok(LlamaContext::from_model(
            self.model_static,
            self.backend,
            context_params,
        )?)
    }

    pub fn decoded_context(&self) -> Result<LlamaContext<'static>> {
        let single_token_capacity: usize = 1;
        let single_sequence_max: i32 = 1;
        let first_sequence_id: i32 = 0;
        let mut context = self.new_context()?;
        let mut batch = LlamaBatch::new(single_token_capacity, single_sequence_max)?;

        batch.add_sequence(&[self.model.token_bos()], first_sequence_id, true)?;
        context.decode(&mut batch)?;

        Ok(context)
    }

    #[must_use]
    pub fn model(&self) -> Arc<LlamaModel> {
        Arc::clone(&self.model)
    }

    pub fn token_classifier(&self) -> Result<SampledTokenClassifier<'static>> {
        Ok(self.model_static.sampled_token_classifier()?)
    }

    pub fn scheduler_context(
        &self,
        inference_parameters: InferenceParameters,
    ) -> Result<Arc<ContinuousBatchSchedulerContext>> {
        self.build_scheduler_context(inference_parameters, None)
    }

    pub fn multimodal_scheduler_context(
        &self,
        inference_parameters: InferenceParameters,
    ) -> Result<Arc<ContinuousBatchSchedulerContext>> {
        let multimodal_projection_path = resolve_model_card_file(&smolvlm2_256m_mmproj())?;
        let multimodal_context = MtmdContext::init_from_file(
            &multimodal_projection_path.to_string_lossy(),
            self.model_static,
            &MtmdContextParams::default(),
        )?;

        self.build_scheduler_context(inference_parameters, Some(Arc::new(multimodal_context)))
    }

    fn build_scheduler_context(
        &self,
        inference_parameters: InferenceParameters,
        multimodal_context: Option<Arc<MtmdContext>>,
    ) -> Result<Arc<ContinuousBatchSchedulerContext>> {
        Ok(Arc::new(ContinuousBatchSchedulerContext {
            agent_name: None,
            chat_template_renderer: Arc::new(ChatTemplateRenderer::new(ChatTemplate {
                content: "Hello {{ name }}!".to_owned(),
            })?),
            desired_slots_total: 1,
            inference_parameters,
            model: Arc::clone(&self.model),
            model_path: self.model_path.clone(),
            multimodal_context,
            token_bos_str: String::new(),
            token_eos_str: String::new(),
            token_nl_str: String::new(),
        }))
    }
}
