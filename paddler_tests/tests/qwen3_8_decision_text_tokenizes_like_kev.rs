#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;

use paddler_test_cluster_harness::model_card::qwen3_8_27b::qwen3_8_27b;

use crate::kev_text_tokenization_reference::KevTextTokenizationReference;

#[test]
fn qwen3_8_decision_text_tokenizes_like_kev() {
    KevTextTokenizationReference {
        model_path: qwen3_8_27b()
            .reference
            .cached_path(&Cache::from_env())
            .expect("the Qwen3.8 model card must be in the Hugging Face cache"),
        reference_fixture: "qwen3_8_tokenizer_reference.json",
    }
    .assert_matched_by_the_decision_text_tokenizer();
}
