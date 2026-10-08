#![cfg(feature = "tests_that_use_llms")]

use serde_json::from_str;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision::decision_slots_minimum::DECISION_SLOTS_MINIMUM;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_test_cluster_harness::agent_config::AgentConfig;
use paddler_test_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_tests::kev_0_8b_desired_state::kev_0_8b_desired_state;
use paddler_tests::kev_parity_record::KevParityRecord;
use paddler_tests::start_cluster::start_cluster;

const KEV_DOCUMENTED_GPU_BF16_DEVIATION: f32 = 0.03;
fn most_probable_option(probabilities: &[f32]) -> Option<usize> {
    probabilities
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(option_index, _)| option_index)
}

#[tokio::test(flavor = "multi_thread")]
async fn kev_decisions_match_the_fp32_reference() {
    let records: Vec<KevParityRecord> = from_str(include_str!(
        "../../fixtures/kev_0_8b_parity_reference.json"
    ))
    .expect("the parity reference must parse");
    let cluster = start_cluster(ClusterParams {
        agents: AgentConfig::uniform(1, DECISION_SLOTS_MINIMUM),
        desired_state: ClusterDesiredState::Apply(Box::new(kev_0_8b_desired_state())),
        ..ClusterParams::default()
    })
    .await
    .expect("the Kev cluster must start");

    for KevParityRecord {
        probabilities,
        questions,
        state,
    } in records
    {
        let collected = cluster
            .decide(
                CancellationToken::new(),
                &RawDecideParams {
                    questions: questions.clone(),
                    state,
                },
            )
            .await
            .expect("the decision must be answered");

        assert!(matches!(collected.terminal_result, DecisionResult::Done(_)));
        assert_eq!(
            collected
                .answers
                .iter()
                .map(|answer| answer.id.as_str())
                .collect::<Vec<_>>(),
            questions
                .iter()
                .map(|question| question.id.as_str())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            collected
                .answers
                .iter()
                .map(|answer| most_probable_option(&answer.probabilities))
                .collect::<Vec<_>>(),
            probabilities
                .iter()
                .map(|reference_probabilities| most_probable_option(reference_probabilities))
                .collect::<Vec<_>>()
        );

        let largest_deviation = collected
            .answers
            .iter()
            .zip(&probabilities)
            .flat_map(|(answer, reference_probabilities)| {
                answer
                    .probabilities
                    .iter()
                    .zip(reference_probabilities)
                    .map(|(served, reference)| (served - reference).abs())
            })
            .fold(0.0_f32, f32::max);

        assert!(
            largest_deviation <= KEV_DOCUMENTED_GPU_BF16_DEVIATION,
            "{largest_deviation}"
        );
    }

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
