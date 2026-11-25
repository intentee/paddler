use serde::Deserialize;

use paddler_messaging::decision_question::DecisionQuestion;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KevParityRecord {
    pub probabilities: Vec<Vec<f32>>,
    pub questions: Vec<DecisionQuestion>,
    pub state: String,
}
