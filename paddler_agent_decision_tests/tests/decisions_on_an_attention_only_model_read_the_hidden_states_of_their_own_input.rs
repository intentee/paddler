#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_decision_tests::answered_decision_probabilities::answered_decision_probabilities;
use paddler_agent_pipeline_harness::model_declaring_decision_metadata::ModelDeclaringDecisionMetadata;
use paddler_cluster_harness::agent_config::AgentConfig;
use paddler_cluster_harness::cluster::Cluster;
use paddler_cluster_harness::cluster_desired_state::ClusterDesiredState;
use paddler_cluster_harness::cluster_params::ClusterParams;
use paddler_messaging::constants::DECISION_SLOTS_MINIMUM;
use paddler_messaging::decision_question::DecisionQuestion;
use paddler_messaging::request_params::decide_params::raw_decide_params::RawDecideParams;
use paddler_model_card_fixture::qwen3_0_6b::qwen3_0_6b;
use paddler_request_params_fixture::sample_decision::sample_decision;

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
    let attention_only_kev_model =
        ModelDeclaringDecisionMetadata::write(qwen3_0_6b(), |gguf_context| {
            gguf_context.set_val_str(c"qwen3.decision.type", c"kev");
            gguf_context.set_val_str(c"qwen3.decision.temperature.choice", c"1.0");
            gguf_context.set_val_str(c"qwen3.decision.temperature.noul", c"1.0");
            gguf_context.set_val_str(c"qwen3.decision.temperature.score", c"1.0");

            Ok(())
        })
        .expect("the attention-only model declaring a kev decision type must be written");
    let cluster = Cluster::start_in_process(ClusterParams {
        agents: AgentConfig::uniform(1, DECISION_SLOTS_MINIMUM.get()),
        desired_state: ClusterDesiredState::Apply(Box::new(
            attention_only_kev_model
                .desired_state()
                .expect("the decision model must lie at an absolute path"),
        )),
        ..ClusterParams::default()
    })
    .await
    .expect("a decision cluster on an attention-only model must start");

    let invoice_before_other_decisions =
        answered_decision_probabilities(&cluster, &sample_decision())
            .await
            .expect("the decision must be answered");

    answered_decision_probabilities(&cluster, &decision_with_many_options())
        .await
        .expect("the decision must be answered");

    let weather_with_the_same_questions = answered_decision_probabilities(
        &cluster,
        &RawDecideParams {
            state: "The weather in Oslo is cold, dark and windy all week.".to_owned(),
            ..sample_decision()
        },
    )
    .await
    .expect("the decision must be answered");
    let invoice_after_other_decisions =
        answered_decision_probabilities(&cluster, &sample_decision())
            .await
            .expect("the decision must be answered");

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
