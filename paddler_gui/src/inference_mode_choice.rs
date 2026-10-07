use std::fmt;

use paddler_messaging::inference_mode::InferenceMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InferenceModeChoice(pub InferenceMode);

impl InferenceModeChoice {
    pub const ALL: [Self; 3] = [
        Self(InferenceMode::TextGeneration),
        Self(InferenceMode::Embeddings),
        Self(InferenceMode::Decision),
    ];
}

impl fmt::Display for InferenceModeChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self.0 {
            InferenceMode::Decision => "Decision",
            InferenceMode::Embeddings => "Embeddings",
            InferenceMode::TextGeneration => "Text generation",
        })
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::inference_mode::InferenceMode;

    use super::InferenceModeChoice;

    #[test]
    fn labels_every_inference_mode() {
        assert_eq!(
            [
                InferenceMode::Decision,
                InferenceMode::Embeddings,
                InferenceMode::TextGeneration,
            ]
            .map(|inference_mode| InferenceModeChoice(inference_mode).to_string()),
            ["Decision", "Embeddings", "Text generation"]
        );
    }
}
