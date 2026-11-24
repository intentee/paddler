#![cfg(feature = "tests_that_use_llms")]

use paddler_messaging::generated_token_result::GeneratedTokenResult;
use paddler_tests::image_question_scheduling::ImageQuestionScheduling;

#[test]
fn a_multimodal_prompt_reaching_a_scheduler_without_a_multimodal_projection_ends_the_generation() {
    assert!(matches!(
        ImageQuestionScheduling::WithoutAMultimodalProjection.first_generated_token_result(),
        Ok(GeneratedTokenResult::MultimodalNotSupported(_))
    ));
}
