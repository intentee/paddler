#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_model::model_error::ModelError;

use paddler_agent_decision::constants::KEV_DECISION_DELIMITERS;
use paddler_agent_decision::decision_delimiter_tokens::DecisionDelimiterTokens;
use paddler_agent_decision::decision_delimiters::DecisionDelimiters;
use paddler_agent_decision::decision_error::DecisionError;
use paddler_cluster_harness::model_card::kev_0_8b::kev_0_8b;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

const DELIMITER_WITH_A_NUL_BYTE: &str = "<|fim_suffix|>\0";

#[test]
fn a_decision_delimiter_containing_a_nul_byte_cannot_be_resolved() {
    let runtime = LlamaRuntime::load(
        &kev_0_8b()
            .cached_path()
            .expect("the kev model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the kev decision model must load");

    assert!(matches!(
        DecisionDelimiterTokens::resolve(
            &*runtime.loaded_llama_model.model.llama_model,
            &DecisionDelimiters {
                decide: DELIMITER_WITH_A_NUL_BYTE,
                ..KEV_DECISION_DELIMITERS
            },
        ),
        Err(DecisionError::DelimiterTokenizationFailed {
            delimiter,
            source: ModelError::TokenizeTextContainsNulByte(_),
        }) if delimiter == DELIMITER_WITH_A_NUL_BYTE
    ));
}
