#![cfg(feature = "tests_that_use_llms")]

use std::io;
use std::path::PathBuf;

use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_error::DecisionError;
use paddler_agent_decision::decision_pipeline::DecisionPipeline;
use paddler_agent_decision::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_agent_decision::general_architecture_metadata::GeneralArchitectureMetadata;
use paddler_agent_runtime::agent_runtime_error::AgentRuntimeError;
use paddler_inference_parameters::model_runtime_parameters::ModelRuntimeParameters;
use paddler_tests::inference_runtime_context_with_unknown_parallelism::inference_runtime_context_with_unknown_parallelism;
use paddler_tests::kev_0_8b_target_path::kev_0_8b_target_path;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_pipeline_refuses_to_start_without_knowing_its_parallelism() {
    assert_eq!(
        DecisionPipeline {
            architecture: GeneralArchitectureMetadata,
            inference_runtime_context: inference_runtime_context_with_unknown_parallelism(
                DECISION_SLOTS_MINIMUM
            ),
            model_path: PathBuf::from("model-that-is-never-read.gguf"),
            model_runtime_parameters: ModelRuntimeParameters::default(),
            pointer_head_path: kev_0_8b_target_path("pointer_head.gguf"),
        }
        .spawn(&CancellationToken::new())
        .await
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(
            DecisionError::AgentRuntime(AgentRuntimeError::AvailableParallelismUnknown(
                io::Error::other("the machine hides how many threads it runs in parallel")
            ))
            .to_string()
        )
    );
}
