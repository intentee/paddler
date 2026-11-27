use std::sync::Arc;

use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_decision::decision_model::DecisionModel;
use llama_cpp_model_capabilities::detected_decisions::DetectedDecisions;

use paddler_agent_model_capabilities::established_model_capabilities::EstablishedModelCapabilities;
use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_model_capabilities::multimodal_projection::MultimodalProjection;
use paddler_inference_parameters::pooling_type::PoolingType;
use paddler_messaging::chat_template_source::ChatTemplateSource;

use crate::llama_runtime::LlamaRuntime;

pub struct EstablishedDecisionCapabilities {
    pub decision_model: DecisionModel,
    pub model_capabilities: Arc<EstablishedModelCapabilities>,
}

impl EstablishedDecisionCapabilities {
    /// # Errors
    ///
    /// Returns an error when the capabilities of the loaded model cannot be established, and when llama.cpp serves no decisions of it.
    pub fn of(llama_runtime: &LlamaRuntime) -> Result<Self> {
        let model_capabilities =
            llama_runtime.establish_model_capabilities(ModelCapabilitiesSettings {
                chat_template_source: ChatTemplateSource::EmbeddedInModel,
                embedding_pooling: PoolingType::Unspecified,
                multimodal_projection: MultimodalProjection::NotConfigured,
            })?;
        let DetectedDecisions::Supported(decision_model) = &model_capabilities.detected.decisions
        else {
            return Err(anyhow!(
                "llama.cpp must serve decisions of the loaded model"
            ));
        };

        Ok(Self {
            decision_model: decision_model.clone(),
            model_capabilities: model_capabilities.clone(),
        })
    }
}
