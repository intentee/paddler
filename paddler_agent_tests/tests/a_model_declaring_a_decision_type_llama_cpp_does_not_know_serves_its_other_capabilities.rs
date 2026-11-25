#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use llama_cpp_bindings_types::decisions_capability::DecisionsCapability;
use llama_cpp_bindings_types::decisions_unsupported_reason::DecisionsUnsupportedReason;
use llama_cpp_bindings_types::model_capabilities::ModelCapabilities;
use llama_cpp_bindings_types::text_generation_capability::TextGenerationCapability;

use paddler_agent_pipeline_harness::model_with_patched_metadata::ModelWithPatchedMetadata;
use paddler_agent_pipeline_harness::serving_pipeline_arbiter::ServingPipelineArbiter;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_model_card_fixture::smolvlm2_256m::smolvlm2_256m;

#[tokio::test(flavor = "multi_thread")]
async fn a_model_declaring_a_decision_type_llama_cpp_does_not_know_serves_its_other_capabilities() {
    let model_declaring_an_unknown_decision_type =
        ModelWithPatchedMetadata::write(smolvlm2_256m(), |gguf_context| {
            gguf_context.set_val_str(c"llama.decision.type", c"not-a-decision-type");

            Ok(())
        })
        .expect("the model declaring an unknown decision type must be written");
    let mut serving_pipeline_arbiter =
        ServingPipelineArbiter::new(1).expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(
            model_declaring_an_unknown_decision_type
                .desired_state()
                .expect("the model must lie at an absolute path"),
        )
        .await
        .expect("the desired state must be served");

    let status = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;

    assert_eq!(status.issues, BTreeSet::new());
    assert!(matches!(
        status.runtime,
        AgentRuntimeStatus::Serving {
            capabilities: ModelCapabilities {
                decisions: DecisionsCapability::Unsupported(
                    DecisionsUnsupportedReason::DecisionTypeUnsupportedByLlamaCpp
                ),
                text_generation: TextGenerationCapability::Supported,
                ..
            },
            slots_total: 1,
        }
    ));

    serving_pipeline_arbiter.shut_down().await;
}
