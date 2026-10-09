#![cfg(feature = "tests_that_use_llms")]

use std::path::PathBuf;

use paddler_tests::kev_0_8b_model_path::KEV_0_8B_MODEL_PATH;

use crate::hugging_face_tokenizer_reference::HuggingFaceTokenizerReference;

#[test]
fn qwen3_5_decision_text_matches_the_hugging_face_tokenizer() {
    HuggingFaceTokenizerReference {
        model_path: PathBuf::from(KEV_0_8B_MODEL_PATH),
        reference_fixture: "qwen3_5_tokenizer_reference.json",
    }
    .assert_matched_by_the_decision_text_tokenizer();
}
