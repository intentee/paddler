#![cfg(feature = "tests_that_use_llms")]

use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;

use crate::kev_text_tokenization_reference::KevTextTokenizationReference;

#[test]
fn qwen3_5_decision_text_tokenizes_like_kev() {
    KevTextTokenizationReference {
        model_path: kev_0_8b_target_path("model.gguf"),
        reference_fixture: "qwen3_5_tokenizer_reference.json",
    }
    .assert_matched_by_the_decision_text_tokenizer();
}
