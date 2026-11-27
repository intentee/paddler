#![cfg(feature = "tests_that_use_llms")]

use std::fs::read_to_string;
use std::mem::discriminant;

use llama_cpp_bindings_types::decision_answer::DecisionAnswer;
use serde_json::from_str;
use tokio_util::sync::CancellationToken;

use paddler_agent_decision_tests::kev_parity_record::KevParityRecord;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::decision_done_discriminant::decision_done_discriminant;
use paddler_cluster_harness::test_cluster_preset::TestClusterPreset;
use paddler_file_fixture::fixture_path::fixture_path;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;

const KEV_DOCUMENTED_GPU_BF16_DEVIATION: f64 = 0.03;

fn most_probable_option(probabilities: &[f64]) -> Option<usize> {
    probabilities
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .map(|(option_index, _)| option_index)
}

fn choice_probabilities(answer: &DecisionAnswer) -> Vec<f64> {
    match answer {
        DecisionAnswer::Choice { probabilities, .. } => probabilities.values().copied().collect(),
        DecisionAnswer::Noul { .. } | DecisionAnswer::Score { .. } => Vec::new(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn kev_decisions_match_the_fp32_reference() {
    let records: Vec<KevParityRecord> = from_str(
        &read_to_string(fixture_path("kev_0_8b_parity_reference.json"))
            .expect("the parity reference must be readable"),
    )
    .expect("the parity reference must parse");
    let cluster = Cluster::start_in_process(TestClusterPreset::Kev0_8b.cluster_params())
        .await
        .expect("the Kev cluster must start");

    for KevParityRecord {
        probabilities,
        request,
    } in records
    {
        let collected = cluster
            .decide(CancellationToken::new(), &request)
            .await
            .expect("the decision must be answered");
        let served_probabilities: Vec<Vec<f64>> = collected
            .answers
            .iter()
            .map(|AnsweredDecisionQuestion { answer, .. }| choice_probabilities(answer))
            .collect();

        assert_eq!(
            discriminant(&collected.terminal_result),
            decision_done_discriminant()
        );
        assert_eq!(
            collected
                .answers
                .iter()
                .map(|answered_question| answered_question.id.as_str())
                .collect::<Vec<_>>(),
            request
                .questions
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            served_probabilities
                .iter()
                .map(|served| most_probable_option(served))
                .collect::<Vec<_>>(),
            probabilities
                .iter()
                .map(|reference_probabilities| most_probable_option(reference_probabilities))
                .collect::<Vec<_>>()
        );

        let largest_deviation = served_probabilities
            .iter()
            .zip(&probabilities)
            .flat_map(|(served_probabilities, reference_probabilities)| {
                assert_eq!(served_probabilities.len(), reference_probabilities.len());

                served_probabilities
                    .iter()
                    .zip(reference_probabilities)
                    .map(|(served, reference)| (served - reference).abs())
            })
            .fold(0.0_f64, f64::max);

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
