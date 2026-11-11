#![cfg(feature = "tests_that_use_llms")]

use paddler_test_cluster_harness::model_card::qwen3_8_27b::qwen3_8_27b;
use paddler_tests::kev_tokenizer_reference_comparison::KevTokenizerReferenceComparison;

#[test]
fn qwen3_8_decision_text_tokenizes_like_kev() {
    let comparison = KevTokenizerReferenceComparison {
        model_path: qwen3_8_27b()
            .cached_path()
            .expect("the Qwen3.8 model card must be in the Hugging Face cache"),
        reference_fixture: "qwen3_8_tokenizer_reference.json",
    };
    let reference = comparison
        .reference()
        .expect("the tokenizer reference must load");

    assert_eq!(
        comparison
            .decision_text_tokenizer_entries(&reference)
            .expect("the decision text tokenizer must tokenize every reference text"),
        reference.entries
    );
}
