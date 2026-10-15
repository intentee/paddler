#![cfg(feature = "tests_that_use_llms")]

use hf_hub::Cache;
use llama_cpp_bindings::context::params::LlamaPoolingType;
use llama_cpp_bindings::error::EmbeddingsError;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_test_cluster_harness::model_card::nomic_embed_text_v1_5::nomic_embed_text_v1_5;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::llama_runtime::LlamaRuntime;

#[test]
fn a_decision_context_for_a_pooling_model_reports_hidden_states_unsupported() {
    let runtime = LlamaRuntime::load(
        &nomic_embed_text_v1_5()
            .reference
            .cached_path(&Cache::from_env())
            .expect("the nomic embedding model must be in the Hugging Face cache"),
        agent_inference_runtime_context(1),
    )
    .expect("the nomic embedding model must load");

    assert_eq!(
        runtime
            .create_decision_llama_context(1)
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            DecisionError::HiddenStatesUnsupported(
                EmbeddingsError::NextnEmbeddingsRequireNonePooling {
                    pooling_type: LlamaPoolingType::Mean,
                }
            )
            .to_string()
        )
    );
}
