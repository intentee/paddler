#![cfg(feature = "tests_that_use_llms")]

use paddler_tests::kev_checkpoint::KevCheckpoint;
use paddler_tests::kev_tokenizer_reference_comparison::KevTokenizerReferenceComparison;

#[test]
fn qwen3_5_decision_text_tokenizes_like_kev() {
    let comparison = KevTokenizerReferenceComparison {
        model_path: KevCheckpoint::Kev0_8b.target_path("model.gguf"),
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
