use anyhow::Context as _;
use anyhow::Result;
use futures_util::FutureExt as _;
use futures_util::TryFutureExt as _;
use futures_util::future::ready;
use tokio_util::sync::CancellationToken;

use llama_cpp_bindings_types::decision_answer::DecisionAnswer;
use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_cluster_harness::collected_decision_results::CollectedDecisionResults;

use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_messaging::decision_result::DecisionResult;
use paddler_messaging::decision_summary::DecisionSummary;
use paddler_request_params_fixture::sample_decision::sample_decision;

const PROBABILITY_SUM_TOLERANCE: f64 = 1e-5;
const SAMPLE_DECISION_INPUT_TOKENS: usize = 83;

fn answers_its_question(answer: &DecisionAnswer, question: &DecisionQuestion) -> bool {
    match (answer, question) {
        (
            DecisionAnswer::Choice { probabilities, .. },
            DecisionQuestion::Choice { criteria, .. },
        ) => probabilities.keys().eq(criteria.keys()),
        (DecisionAnswer::Noul { noul }, DecisionQuestion::Noul { .. }) => {
            (0.0..=1.0).contains(noul)
        }
        (DecisionAnswer::Score { probabilities, .. }, DecisionQuestion::Score { criteria, .. }) => {
            probabilities.len() == criteria.len()
        }
        _ => false,
    }
}

fn probabilities_sum_to_one(answer: &DecisionAnswer) -> bool {
    match answer {
        DecisionAnswer::Choice { probabilities, .. }
        | DecisionAnswer::Score { probabilities, .. } => {
            (probabilities.values().sum::<f64>() - 1.0).abs() < PROBABILITY_SUM_TOLERANCE
        }
        DecisionAnswer::Noul { .. } => true,
    }
}

fn sample_decision_answered(
    CollectedDecisionResults {
        answers,
        terminal_result,
    }: &CollectedDecisionResults,
    DecisionRequest { questions, .. }: &DecisionRequest,
) -> Result<()> {
    let answered_question_ids: Vec<&str> = answers
        .iter()
        .map(|AnsweredDecisionQuestion { id, .. }| id.as_str())
        .collect();
    let question_ids: Vec<&str> = questions.keys().map(String::as_str).collect();

    (answered_question_ids == question_ids)
        .then_some(())
        .context(format!(
            "the answers to {answered_question_ids:?} must match the questions {question_ids:?}"
        ))
        .and_then(|()| {
            answers
                .iter()
                .zip(questions.values())
                .all(|(AnsweredDecisionQuestion { answer, .. }, question)| {
                    answers_its_question(answer, question) && probabilities_sum_to_one(answer)
                })
                .then_some(())
                .context(format!(
                    "every answer must answer its question with probabilities summing to one: {answers:?}"
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
        Cluster::start_in_process(self.cluster_params)
            .map(|started| started.context("the cluster must start"))
            .and_then(decide_the_sample_decision)
            .await
    }
}
