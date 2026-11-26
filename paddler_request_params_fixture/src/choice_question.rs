use llama_cpp_bindings_types::decision_question::DecisionQuestion;
use serde_json::Value;
use serde_json::json;

#[must_use]
pub fn choice_question(instructions: &str, options: &[&str]) -> DecisionQuestion {
    DecisionQuestion::Choice {
        criteria: options
            .iter()
            .map(|option| ((*option).to_owned(), Value::Null))
            .collect(),
        instructions: json!(instructions),
    }
}
