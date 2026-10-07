use std::time::UNIX_EPOCH;

use serde_json::from_value;
use serde_json::json;

use paddler_inference_parameters::embedding_parameters::EmbeddingParameters;
use paddler_messaging::agent_inference_settings::AgentInferenceSettings;
use paddler_messaging::inference_mode::InferenceMode;
use paddler_openai_translation::openai_translation_error::OpenAITranslationError;
use paddler_openai_translation::responses_request::ResponsesRequest;

#[test]
fn a_responses_request_to_a_cluster_of_another_inference_mode_is_refused() {
    let responses_request: ResponsesRequest =
        from_value(json!({"model": "test-model", "input": "hi"}))
            .expect("the responses request must deserialize");

    assert!(matches!(
        responses_request.translate(
            UNIX_EPOCH,
            &AgentInferenceSettings::Embeddings(EmbeddingParameters::default())
        ),
        Err(OpenAITranslationError::InferenceModeMismatch { inference_mode })
            if inference_mode == InferenceMode::Embeddings
    ));
}
