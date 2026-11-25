use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_answer::DecisionAnswer;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;

use crate::round_probability::round_probability;

fn rounded_probabilities(probabilities: IndexMap<String, f64>) -> Map<String, Value> {
    probabilities
        .into_iter()
        .map(|(key, probability)| (key, Value::from(round_probability(probability))))
        .collect()
}

pub struct SystemOneAnswer {
    pub decision_answer: DecisionAnswer,
}

impl SystemOneAnswer {
    #[must_use]
    pub fn into_value(self) -> Value {
        match self.decision_answer {
            DecisionAnswer::Choice {
                choice,
                confidence,
                probabilities,
            } => json!({
                "type": "choice",
                "choice": choice,
                "confidence": round_probability(confidence),
                "probabilities": rounded_probabilities(probabilities),
            }),
            DecisionAnswer::Noul { noul } => {
                json!({ "type": "noul", "noul": round_probability(noul) })
            }
            DecisionAnswer::Score {
                confidence,
                legend,
                probabilities,
                score,
            } => json!({
                "type": "score",
                "score": round_probability(score),
                "legend": legend,
                "probabilities": rounded_probabilities(probabilities),
                "confidence": round_probability(confidence),
            }),
        }
    }
}
