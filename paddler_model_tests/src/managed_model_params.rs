use paddler_types::huggingface_model_reference::HuggingFaceModelReference;
use paddler_types::inference_parameters::InferenceParameters;

pub struct ManagedModelParams {
    pub inference_parameters: InferenceParameters,
    pub model: HuggingFaceModelReference,
    pub multimodal_projection: Option<HuggingFaceModelReference>,
    pub slots: i32,
}
