#![cfg(feature = "tests_that_use_llms")]

use paddler_test_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_tests::kev_tokenizer_reference_comparison::KevTokenizerReferenceComparison;

#[test]
fn qwen3_5_decision_text_tokenizes_like_kev() {
    let comparison = KevTokenizerReferenceComparison {
        model_path: kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
        reference_fixture: "qwen3_5_tokenizer_reference.json",
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
