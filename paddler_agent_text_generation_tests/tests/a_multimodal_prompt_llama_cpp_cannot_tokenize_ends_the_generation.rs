#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings_sys::bindings::LLAMA_RS_FAULT_POINT_MTMD_TOKENIZE_FROM_PARTS_THROWS;
use llama_cpp_fault_injection::armed_fault::ArmedFault;

use paddler_agent_text_generation_tests::image_question_scheduling::ImageQuestionScheduling;
use paddler_messaging::generated_token_result::GeneratedTokenResult;

#[test]
fn a_multimodal_prompt_llama_cpp_cannot_tokenize_ends_the_generation() {
    let _multimodal_prompt_fails =
        ArmedFault::at(LLAMA_RS_FAULT_POINT_MTMD_TOKENIZE_FROM_PARTS_THROWS);

    assert!(matches!(
        ImageQuestionScheduling::WithAMultimodalProjection.first_generated_token_result(),
        Ok(GeneratedTokenResult::MultimodalTokenizationFailed(_))
    ));
}
