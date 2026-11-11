use anyhow::Context as _;
use anyhow::Result;
use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use tokio_util::sync::CancellationToken;

use paddler_messaging::decision_answer::DecisionAnswer;
use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_test_cluster_harness::cluster::Cluster;
use paddler_test_cluster_harness::cluster_params::ClusterParams;
use paddler_test_cluster_harness::collected_decision_results::CollectedDecisionResults;

use crate::sample_decision::sample_decision;
use crate::start_cluster::start_cluster;

const PROBABILITY_SUM_TOLERANCE: f32 = 1e-5;
const SAMPLE_DECISION_INPUT_TOKENS: usize = 85;

fn sample_decision_answered(
    CollectedDecisionResults {
        answers,
        terminal_result,
    }: &CollectedDecisionResults,
    RawDecideParams { questions, .. }: &RawDecideParams,
) -> Result<()> {
    let answered_question_ids: Vec<&str> = answers
        .iter()
        .map(|DecisionAnswer { id, .. }| id.as_str())
        .collect();
    let question_ids: Vec<&str> = questions
        .iter()
        .map(|DecisionQuestion { id, .. }| id.as_str())
        .collect();
    let answered_option_counts: Vec<usize> = answers
        .iter()
        .map(|DecisionAnswer { probabilities, .. }| probabilities.len())
        .collect();
    let option_counts: Vec<usize> = questions
        .iter()
        .map(|DecisionQuestion { options, .. }| options.len())
        .collect();

    (answered_question_ids == question_ids && answered_option_counts == option_counts)
        .then_some(())
        .context(format!(
            "the answers to {answered_question_ids:?} with {answered_option_counts:?} probabilities must match the questions {question_ids:?} with {option_counts:?} options"
        ))
        .and_then(|()| {
            answers
                .iter()
                .all(|DecisionAnswer { probabilities, .. }| {
                    (probabilities.iter().sum::<f32>() - 1.0).abs() < PROBABILITY_SUM_TOLERANCE
                })
                .then_some(())
                .context(format!(
                    "the probabilities of every answer must sum to one: {answers:?}"
                ))
        })
        .and_then(|()| {
            matches!(
                terminal_result,
                DecisionResult::Done(DecisionSummary { input_tokens, .. })
                    if *input_tokens == SAMPLE_DECISION_INPUT_TOKENS
            )
            .then_some(())
            .context(format!(
                "the sample decision must evaluate {SAMPLE_DECISION_INPUT_TOKENS} input tokens, but it ended with {terminal_result:?}"
            ))
        })
}

async fn decide_the_sample_decision(cluster: Cluster) -> Result<()> {
    let decision = sample_decision();

    cluster
        .decide(CancellationToken::new(), &decision)
        .map(|decided| decided.context("the decision must be accepted"))
        .and_then(|collected| ready(sample_decision_answered(&collected, &decision)))
        .and_then(|()| {
            cluster
                .shutdown()
                .map(|shut_down| shut_down.context("the cluster must shut down cleanly"))
        })
        .await
}

pub struct SampleDecisionAnswersCheck {
    pub cluster_params: ClusterParams,
}

impl SampleDecisionAnswersCheck {
    /// # Errors
    ///
    /// Returns an error when the cluster cannot start, when the sample decision is not accepted, when its answers do not match its questions, do not sum to one, or do not evaluate every input token, and when the cluster does not shut down cleanly.
    pub async fn ensure_for_the_sample_decision(self) -> Result<()> {
        start_cluster(self.cluster_params)
            .map(|started| started.context("the cluster must start"))
            .and_then(decide_the_sample_decision)
            .await
    }
}
