use serde::Deserialize;

#[derive(Deserialize)]
pub struct OpenAIResponsesReasoning {
    #[serde(default)]
    pub effort: Option<String>,
}

impl OpenAIResponsesReasoning {
    #[must_use]
    pub fn enables_thinking(&self) -> bool {
        self.effort.as_deref() != Some("none")
    }
}
