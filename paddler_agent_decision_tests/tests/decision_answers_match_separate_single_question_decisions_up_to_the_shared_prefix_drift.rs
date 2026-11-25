#![cfg(feature = "tests_that_use_llms")]

use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_answer::DecisionAnswer;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use tokio_util::sync::CancellationToken;

use paddler_cluster_harness::decision_cluster_params::DecisionClusterParams;
use paddler_messaging::answered_decision_question::AnsweredDecisionQuestion;
use paddler_request_params_fixture::sample_decision::sample_decision;

const SHARED_PREFIX_DRIFT: f64 = 2e-3;

fn within_the_shared_prefix_drift(joint_value: f64, separate_value: f64) -> bool {
    (joint_value - separate_value).abs() <= SHARED_PREFIX_DRIFT
}

fn probabilities_agree(
    joint_probabilities: &IndexMap<String, f64>,
    separate_probabilities: &IndexMap<String, f64>,
) -> bool {
    joint_probabilities.len() == separate_probabilities.len()
        && joint_probabilities.iter().zip(separate_probabilities).all(
            |((joint_option, joint_probability), (separate_option, separate_probability))| {
                joint_option == separate_option
                    && within_the_shared_prefix_drift(*joint_probability, *separate_probability)
            },
        )
}

fn answered_alike(
    AnsweredDecisionQuestion {
        answer: joint_answer,
        id: joint_id,
    }: &AnsweredDecisionQuestion,
    AnsweredDecisionQuestion {
        answer: separate_answer,
        id: separate_id,
    }: &AnsweredDecisionQuestion,
) -> bool {
    joint_id == separate_id
        && match (joint_answer, separate_answer) {
            (
                DecisionAnswer::Choice {
                    choice: joint_choice,
                    confidence: joint_confidence,
                    probabilities: joint_probabilities,
                },
                DecisionAnswer::Choice {
                    choice: separate_choice,
                    confidence: separate_confidence,
                    probabilities: separate_probabilities,
                },
            ) => {
                joint_choice == separate_choice
                    && within_the_shared_prefix_drift(*joint_confidence, *separate_confidence)
                    && probabilities_agree(joint_probabilities, separate_probabilities)
            }
            (
                DecisionAnswer::Noul { noul: joint_noul },
                DecisionAnswer::Noul {
                    noul: separate_noul,
                },
            ) => within_the_shared_prefix_drift(*joint_noul, *separate_noul),
            (
                DecisionAnswer::Choice { .. }
                | DecisionAnswer::Noul { .. }
                | DecisionAnswer::Score { .. },
                _,
            ) => false,
        }
}

#[tokio::test(flavor = "multi_thread")]
async fn decision_answers_match_separate_single_question_decisions_up_to_the_shared_prefix_drift() {
    let cluster = DecisionClusterParams::default()
        .start_cluster()
        .await
        .expect("the cluster must start");
    let decision = sample_decision();

    let joint_answers = cluster
        .decide(CancellationToken::new(), &decision)
        .await
        .expect("the joint decision must be accepted")
        .answers;
    let mut separate_answers = Vec::new();

    for (question_id, question) in &decision.questions {
        separate_answers.extend(
            cluster
                .decide(
                    CancellationToken::new(),
                    &DecisionRequest {
                        questions: IndexMap::from([(question_id.clone(), question.clone())]),
                        ..decision.clone()
                    },
                )
                .await
                .expect("a single question decision must be accepted")
                .answers,
        );
    }

    assert!(
        joint_answers.len() == separate_answers.len()
            && joint_answers.iter().zip(&separate_answers).all(
                |(joint_answer, separate_answer)| answered_alike(joint_answer, separate_answer)
            ),
        "{joint_answers:?} differs from {separate_answers:?}"
    );

    cluster
        .shutdown()
        .await
        .expect("the cluster must shut down cleanly");
}
