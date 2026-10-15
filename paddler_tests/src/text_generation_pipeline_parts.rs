use std::path::PathBuf;

use tokio_util::sync::CancellationToken;

use paddler_agent_runtime::inference_runtime_context::InferenceRuntimeContext;
use paddler_agent_text_generation::multimodal_projection::MultimodalProjection;
use paddler_agent_text_generation::provides_media_marker::ProvidesMediaMarker;
use paddler_agent_text_generation::text_generation_pipeline::TextGenerationPipeline;
use paddler_agent_text_generation::text_generation_settings::TextGenerationSettings;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_inference_parameters::sampling_parameters::SamplingParameters;
use paddler_messaging::chat_template_source::ChatTemplateSource;
use paddler_messaging::multimodal_settings::MultimodalSettings;

pub struct TextGenerationPipelineParts<TMediaMarker: ProvidesMediaMarker> {
    pub chat_template_source: ChatTemplateSource,
    pub inference_runtime_context: InferenceRuntimeContext,
    pub media_marker: TMediaMarker,
    pub model_path: PathBuf,
    pub multimodal_projection: MultimodalProjection,
}

impl<TMediaMarker: ProvidesMediaMarker> TextGenerationPipelineParts<TMediaMarker> {
    pub fn into_text_generation_pipeline(self) -> TextGenerationPipeline<TMediaMarker> {
        let Self {
            chat_template_source,
            inference_runtime_context,
            media_marker,
            model_path,
            multimodal_projection,
        } = self;

        TextGenerationPipeline {
            inference_runtime_context,
            media_marker,
            model_path,
            model_runtime_parameters: ModelRuntimeParameters::default(),
            text_generation_settings: TextGenerationSettings {
                chat_template_source,
                image_resize_to_fit: MultimodalSettings::default().image_resize_to_fit,
                multimodal_projection,
                sampling_parameters: SamplingParameters::default(),
            },
        }
    }

    pub async fn spawn_refusal(self) -> Option<String> {
        self.into_text_generation_pipeline()
            .spawn(&CancellationToken::new())
            .await
            .err()
            .as_ref()
            .map(ToString::to_string)
    }
}
