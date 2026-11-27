use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::json;

use crate::choice_question::choice_question;

#[must_use]
pub fn weather_and_season_decision() -> DecisionRequest {
    DecisionRequest {
        images: Vec::new(),
        questions: IndexMap::from([
            (
                "weather".to_owned(),
                choice_question(
                    "What is the weather like?",
                    &["sunny", "raining", "snowing"],
                ),
            ),
            (
                "season".to_owned(),
                choice_question("What season is it?", &["summer", "winter"]),
            ),
        ]),
        state: json!("Thick snowflakes have been falling on the frozen street all morning."),
    }
}
