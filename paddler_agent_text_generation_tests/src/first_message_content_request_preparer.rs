use std::path::PathBuf;

use anyhow::Result;

use paddler_agent_model_capabilities::model_capabilities_settings::ModelCapabilitiesSettings;
use paddler_agent_model_capabilities::multimodal_projection::MultimodalProjection;
use paddler_agent_pipeline_harness::llama_runtime::LlamaRuntime;
use paddler_agent_text_generation::generation_request_preparer::GenerationRequestPreparer;
use paddler_balancer_desired_state_fixture::first_message_content_chat_template::first_message_content_chat_template;
use paddler_messaging::chat_template_source::ChatTemplateSource;

use crate::default_model_capabilities_settings::default_model_capabilities_settings;
use crate::model_capabilities_request_preparer::ModelCapabilitiesRequestPreparer;

pub struct FirstMessageContentRequestPreparer<'runtime> {
    pub llama_runtime: &'runtime LlamaRuntime,
    pub sequence_context_size: u32,
}

impl FirstMessageContentRequestPreparer<'_> {
    /// # Errors
    ///
    /// Returns an error when llama.cpp cannot prepare the first message content chat template, and when the capabilities of the loaded model cannot be established.
    pub fn into_generation_request_preparer(self) -> Result<GenerationRequestPreparer> {
        self.into_preparer_with_multimodal_projection(MultimodalProjection::NotConfigured)
    }

    /// # Errors
    ///
    /// Returns an error when llama.cpp cannot prepare the first message content chat template, when the multimodal projection at `multimodal_projection_path` cannot load, and when the capabilities of the loaded model cannot be established.
    pub fn into_multimodal_generation_request_preparer(
        self,
        multimodal_projection_path: PathBuf,
    ) -> Result<GenerationRequestPreparer> {
        self.into_preparer_with_multimodal_projection(MultimodalProjection::File(
            multimodal_projection_path,
        ))
    }

    fn into_preparer_with_multimodal_projection(
        self,
        multimodal_projection: MultimodalProjection,
    ) -> Result<GenerationRequestPreparer> {
        let Self {
            llama_runtime,
            sequence_context_size,
        } = self;

        ModelCapabilitiesRequestPreparer {
            llama_runtime,
            model_capabilities_settings: ModelCapabilitiesSettings {
                chat_template_source: ChatTemplateSource::Override(
                    first_message_content_chat_template(),
                ),
                multimodal_projection,
                ..default_model_capabilities_settings()
            },
            sequence_context_size,
        }
        .into_generation_request_preparer()
    }
}
