use serde::Deserialize;

use paddler_messaging::grammar_constraint::GrammarConstraint;

use crate::compatibility::openai_service::openai_responses_text_format::OpenAIResponsesTextFormat;

#[derive(Deserialize)]
pub struct OpenAIResponsesTextParam {
    #[serde(default)]
    pub format: Option<OpenAIResponsesTextFormat>,
}

impl OpenAIResponsesTextParam {
    #[must_use]
    pub fn into_grammar_constraint(self) -> Option<GrammarConstraint> {
        match self.format {
            Some(OpenAIResponsesTextFormat::JsonSchema { schema }) => {
                Some(GrammarConstraint::JsonSchema {
                    schema: schema.to_string(),
                })
            }
            Some(OpenAIResponsesTextFormat::Text) | None => None,
        }
    }
}
