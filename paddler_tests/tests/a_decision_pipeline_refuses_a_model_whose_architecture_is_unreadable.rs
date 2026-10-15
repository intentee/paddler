#![cfg(feature = "tests_that_use_llms")]

use llama_cpp_bindings::error::MetaValError;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_decision::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_tests::agent_inference_runtime_context::agent_inference_runtime_context;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;
use paddler_tests::model_architecture_that_is_unreadable::ModelArchitectureThatIsUnreadable;
use paddler_tests::model_architecture_that_is_unreadable::UNREADABLE_ARCHITECTURE_RETURN_CODE;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_pipeline_refuses_a_model_whose_architecture_is_unreadable() {
    assert_eq!(
        DecisionPipeline {
            architecture: ModelArchitectureThatIsUnreadable,
            inference_runtime_context: agent_inference_runtime_context(DECISION_SLOTS_MINIMUM),
            model_path: kev_0_8b_target_path("model.gguf"),
            model_runtime_parameters: ModelRuntimeParameters::default(),
            pointer_head_path: kev_0_8b_target_path("pointer_head.gguf"),
        }
        .spawn(&CancellationToken::new())
        .await
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(
            DecisionError::ModelArchitectureUnreadable(MetaValError::NegativeReturn(
                UNREADABLE_ARCHITECTURE_RETURN_CODE
            ))
            .to_string()
        )
    );
}
