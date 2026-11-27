use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::Value;
use serde_json::json;

#[must_use]
pub fn snowy_weather_decision() -> DecisionRequest {
    DecisionRequest {
        images: Vec::new(),
        questions: IndexMap::from([(
            "weather".to_owned(),
            DecisionQuestion::Choice {
                criteria: IndexMap::from([
                    ("sunny".to_owned(), Value::Null),
                    ("raining".to_owned(), Value::Null),
                    ("snowing".to_owned(), Value::Null),
                ]),
                instructions: json!("What is the weather like?"),
            },
        )]),
        state: json!("Thick snowflakes have been falling on the frozen street all morning."),
    }
}
