#![cfg(feature = "tests_that_use_llms")]

use anyhow::Result;
use tokio_util::sync::CancellationToken;

use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::model_card::qwen3_0_6b::qwen3_0_6b;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

async fn answered_probabilities(
    cluster: &Cluster,
    decision: &RawDecideParams,
) -> Result<Vec<Vec<f32>>> {
    cluster
        .decide(CancellationToken::new(), decision)
        .await
        .map(|collected_decision_results| {
            collected_decision_results
                .answers
                .into_iter()
                .map(|answer| answer.probabilities)
                .collect()
        })
}

fn decision_with_many_options() -> RawDecideParams {
    RawDecideParams {
        questions: vec![DecisionQuestion {
            id: "weekday".to_owned(),
            instructions: "Which day was the parcel delivered?".to_owned(),
            options: [
                "monday",
                "tuesday",
                "wednesday",
                "thursday",
                "friday",
                "saturday",
                "sunday",
            ]
            .iter()
            .map(|weekday| (*weekday).to_owned())
            .collect(),
        }],
        state: "The parcel arrived on the last working day of the week.".to_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn decisions_on_an_attention_only_model_read_the_hidden_states_of_their_own_input() {
    let cluster = start_decision_cluster(DecisionClusterParams {
        model_card: qwen3_0_6b(),
        ..DecisionClusterParams::default()
    })
    .await
    .expect("a decision cluster on an attention-only model must start");

    let invoice_before_other_decisions = answered_probabilities(&cluster, &sample_decision())
        .await
        .expect("the invoice decision must be answered");

    answered_probabilities(&cluster, &decision_with_many_options())
        .await
        .expect("the weekday decision must be answered");

    let weather_with_the_same_questions = answered_probabilities(
        &cluster,
        &RawDecideParams {
            state: "The weather in Oslo is cold, dark and windy all week.".to_owned(),
            ..sample_decision()
        },
    )
    .await
    .expect("the weather decision must be answered");
    let invoice_after_other_decisions = answered_probabilities(&cluster, &sample_decision())
        .await
        .expect("the repeated invoice decision must be answered");

    assert_eq!(
        invoice_before_other_decisions,
        invoice_after_other_decisions
    );
    assert_ne!(
        invoice_before_other_decisions,
        weather_with_the_same_questions
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
