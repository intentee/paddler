#![cfg(feature = "tests_that_use_llms")]

use std::collections::BTreeSet;

use paddler_cluster_harness::model_card::smolvlm2_256m::smolvlm2_256m;
use paddler_messaging::agent_decisions_serving::AgentDecisionsServing;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;
use paddler_messaging::agent_runtime_status::AgentRuntimeStatus;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::produces_snapshot::ProducesSnapshot as _;
use paddler_tests::model_declaring_decision_metadata::ModelDeclaringDecisionMetadata;
use paddler_tests::serving_pipeline_arbiter::ServingPipelineArbiter;

#[tokio::test(flavor = "multi_thread")]
async fn agent_reports_a_decision_model_of_an_unsupported_type() {
    let lev_decision_model =
        ModelDeclaringDecisionMetadata::write(smolvlm2_256m(), |gguf_context| {
            gguf_context.set_val_str(c"llama.decision.type", c"lev");

            Ok(())
        })
        .expect("the model declaring a lev decision type must be written");
    let mut serving_pipeline_arbiter = ServingPipelineArbiter::new(DECISION_SLOTS_MINIMUM.get())
        .expect("the model downloader must start");

    serving_pipeline_arbiter
        .serve(
            lev_decision_model
                .desired_state()
                .expect("the decision model must lie at an absolute path"),
        )
        .await
        .expect("the desired state must be served");

    let status = serving_pipeline_arbiter
        .inference_runtime_context
        .slot_aggregated_status
        .make_snapshot()
        .status;

    assert_eq!(
        status.issues,
        BTreeSet::from([AgentIssue::DecisionTypeIsUnsupported(ModelPath::from(
            lev_decision_model.model_path().as_path()
        ))])
    );
    assert_eq!(
        status.runtime,
        AgentRuntimeStatus::Serving {
            decisions: AgentDecisionsServing::DecisionModelNotServed,
            slots_total: u64::from(DECISION_SLOTS_MINIMUM.get()),
        }
    );

    serving_pipeline_arbiter.shut_down().await;
}
