use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use llama_cpp_bindings_types::decision_request::DecisionRequest;
use serde_json::json;

use crate::snowy_weather_decision::snowy_weather_decision;

#[must_use]
pub fn snowy_weather_and_freezing_decision() -> DecisionRequest {
    let mut request = snowy_weather_decision();

    request.questions.insert(
        "freezing".to_owned(),
        DecisionQuestion::Noul {
            criteria: IndexMap::new(),
            instructions: json!("Is it below freezing outside?"),
        },
    );

    request
}
