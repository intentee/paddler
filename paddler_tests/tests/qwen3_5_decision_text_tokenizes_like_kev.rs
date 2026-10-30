#![cfg(feature = "tests_that_use_llms")]

use paddler_tests::kev_checkpoint::KevCheckpoint;

use crate::kev_tokenizer_reference_comparison::KevTokenizerReferenceComparison;

#[test]
fn qwen3_5_decision_text_tokenizes_like_kev() {
    KevTokenizerReferenceComparison {
        model_path: KevCheckpoint::Kev0_8b.target_path("model.gguf"),
        reference_fixture: "qwen3_5_tokenizer_reference.json",
    }
    .assert_matched_by_the_decision_text_tokenizer();
}
