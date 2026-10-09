use serde::Deserialize;

use paddler_messaging::grammar_constraint::GrammarConstraint;

use crate::responses_text_format::ResponsesTextFormat;

#[derive(Deserialize)]
pub struct ResponsesTextParam {
    #[serde(default)]
    pub format: Option<ResponsesTextFormat>,
}

impl ResponsesTextParam {
    #[must_use]
    pub fn into_grammar_constraint(self) -> Option<GrammarConstraint> {
        match self.format {
            Some(ResponsesTextFormat::JsonSchema { schema }) => {
                Some(GrammarConstraint::JsonSchema {
                    schema: schema.to_string(),
                })
            }
            Some(ResponsesTextFormat::Text) | None => None,
        }
    }
}
