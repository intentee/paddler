#![cfg(feature = "tests_that_use_llms")]

use tokio_util::sync::CancellationToken;

use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_tests::decision_cluster_params::DecisionClusterParams;
use paddler_tests::sample_decision::sample_decision;
use paddler_tests::start_decision_cluster::start_decision_cluster;

const PROBABILITY_SUM_TOLERANCE: f32 = 1e-5;

pub struct EveryDecisionQuestionAnsweredInOrder {
    pub decision_cluster_params: DecisionClusterParams,
}

impl EveryDecisionQuestionAnsweredInOrder {
    pub async fn assert_for_the_sample_decision(self) {
        let cluster = start_decision_cluster(self.decision_cluster_params)
            .await
            .expect("the cluster must start");
        let decision = sample_decision();

        let collected = cluster
            .decide(CancellationToken::new(), &decision)
            .await
            .expect("the decision must be accepted");

        assert_eq!(
            collected
                .answers
                .iter()
                .map(|answer| (answer.id.as_str(), answer.probabilities.len()))
                .collect::<Vec<_>>(),
            decision
                .questions
                .iter()
                .map(|question| (question.id.as_str(), question.options.len()))
                .collect::<Vec<_>>()
        );
        assert!(collected.answers.iter().all(|answer| {
            (answer.probabilities.iter().sum::<f32>() - 1.0).abs() < PROBABILITY_SUM_TOLERANCE
        }));
        assert!(matches!(
            collected.terminal_result,
            DecisionResult::Done(DecisionSummary { input_tokens, .. }) if input_tokens > 0
        ));

        cluster
            .shutdown()
            .await
            .expect("the cluster must shut down cleanly");
    }
}
