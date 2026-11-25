#![cfg(feature = "tests_that_use_llms")]

use paddler_agent_decision_tests::chose_option::chose_option;
use paddler_agent_decision_tests::decided_by_a_cluster_serving::decided_by_a_cluster_serving;
use paddler_agent_decision_tests::snowy_weather_decision::snowy_weather_decision;
use paddler_model_card_fixture::d1_2b::d1_2b;

#[tokio::test(flavor = "multi_thread")]
async fn a_decision_model_reading_grouped_label_logits_decides_the_weather_its_state_describes() {
    let collected =
        decided_by_a_cluster_serving(d1_2b().into_desired_state(), &snowy_weather_decision())
            .await
            .expect("the decision must be answered");

    assert!(
        chose_option(&collected.answers, "weather", "snowing"),
        "answers were {:?}",
        collected.answers
    );
}
