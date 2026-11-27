use indexmap::IndexMap;
use llama_cpp_bindings_types::decision_answer::DecisionAnswer;
use serde::Serialize;
use serde_json::Value;

use crate::round_probability::round_probability;

fn rounded_probabilities(probabilities: IndexMap<String, f64>) -> IndexMap<String, f64> {
    probabilities
        .into_iter()
        .map(|(key, probability)| (key, round_probability(probability)))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase", tag = "type")]
pub enum SystemOneAnswer {
    Choice {
        choice: String,
        confidence: f64,
        probabilities: IndexMap<String, f64>,
    },
    Noul {
        noul: f64,
    },
    Score {
        score: f64,
        legend: IndexMap<String, Value>,
        probabilities: IndexMap<String, f64>,
        confidence: f64,
    },
}

impl From<DecisionAnswer> for SystemOneAnswer {
    fn from(decision_answer: DecisionAnswer) -> Self {
        match decision_answer {
            DecisionAnswer::Choice {
                choice,
                confidence,
                probabilities,
            } => Self::Choice {
                choice,
                confidence: round_probability(confidence),
                probabilities: rounded_probabilities(probabilities),
            },
            DecisionAnswer::Noul { noul } => Self::Noul {
                noul: round_probability(noul),
            },
            DecisionAnswer::Score {
                confidence,
                legend,
                probabilities,
                score,
            } => Self::Score {
                score: round_probability(score),
                legend,
                probabilities: rounded_probabilities(probabilities),
                confidence: round_probability(confidence),
            },
        }
    }
}
