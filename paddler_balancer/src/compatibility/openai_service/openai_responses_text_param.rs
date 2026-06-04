use anyhow::Context as _;
use anyhow::Result;
use paddler_messaging::grammar_constraint::GrammarConstraint;
use serde::Deserialize;

use crate::compatibility::openai_service::openai_responses_text_format::OpenAIResponsesTextFormat;

#[derive(Deserialize)]
pub struct OpenAIResponsesTextParam {
    #[serde(default)]
    pub format: Option<OpenAIResponsesTextFormat>,
}

impl OpenAIResponsesTextParam {
    pub fn into_grammar_constraint(self) -> Result<Option<GrammarConstraint>> {
        match self.format {
            Some(OpenAIResponsesTextFormat::JsonSchema { schema }) => {
                Ok(Some(GrammarConstraint::JsonSchema {
                    schema: serde_json::to_string(&schema)
                        .context("serializing responses text.format json schema")?,
                }))
            }
            Some(OpenAIResponsesTextFormat::Text | OpenAIResponsesTextFormat::Unsupported)
            | None => Ok(None),
        }
    }
}
