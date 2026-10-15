use serde_json::Map;
use serde_json::Value;
use serde_json::json;

#[derive(Clone, Debug, PartialEq)]
pub enum SystemOneAnswer {
    Choice {
        choice: String,
        confidence: f64,
        probabilities: Map<String, Value>,
    },
    Noul {
        noul: f64,
    },
    Score {
        score: f64,
        legend: Map<String, Value>,
        probabilities: Map<String, Value>,
        confidence: f64,
    },
}

impl SystemOneAnswer {
    #[must_use]
    pub fn into_value(self) -> Value {
        match self {
            Self::Choice {
                choice,
                confidence,
                probabilities,
            } => json!({
                "type": "choice",
                "choice": choice,
                "confidence": confidence,
                "probabilities": probabilities,
            }),
            Self::Noul { noul } => json!({ "type": "noul", "noul": noul }),
            Self::Score {
                score,
                legend,
                probabilities,
                confidence,
            } => json!({
                "type": "score",
                "score": score,
                "legend": legend,
                "probabilities": probabilities,
                "confidence": confidence,
            }),
        }
    }
}
