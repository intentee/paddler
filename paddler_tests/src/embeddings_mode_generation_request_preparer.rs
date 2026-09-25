use std::sync::Arc;

use llama_cpp_bindings::model::LlamaModel;
use paddler_agent::generation_request_preparer::GenerationRequestPreparer;
use paddler_agent::image_input::ImageInput;
use paddler_agent::prompt_tokenizer::PromptTokenizer;
use paddler_agent::token_generation::TokenGeneration;
use paddler_messaging::inference_parameters::InferenceParameters;

#[must_use]
pub fn embeddings_mode_generation_request_preparer(
    model: Arc<LlamaModel>,
) -> GenerationRequestPreparer {
    let InferenceParameters {
        context_size,
        image_resize_to_fit,
        ..
    } = InferenceParameters::default();

    GenerationRequestPreparer {
        image_input: ImageInput::Unsupported,
        image_resize_to_fit,
        model: model.clone(),
        prompt_tokenizer: PromptTokenizer {
            model,
            sequence_context_size: context_size,
        },
        token_generation: TokenGeneration::DisabledForEmbeddings,
    }
}
