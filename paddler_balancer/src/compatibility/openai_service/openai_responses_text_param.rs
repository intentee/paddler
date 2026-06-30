use paddler_messaging::grammar_constraint::GrammarConstraint;
use serde::Deserialize;

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
            Some(OpenAIResponsesTextFormat::Text | OpenAIResponsesTextFormat::Unsupported)
            | None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use paddler_messaging::grammar_constraint::GrammarConstraint;
    use serde_json::json;

    use super::OpenAIResponsesTextParam;
    use crate::compatibility::openai_service::openai_responses_text_format::OpenAIResponsesTextFormat;

    #[test]
    fn no_format_yields_no_constraint() {
        assert!(
            OpenAIResponsesTextParam { format: None }
                .into_grammar_constraint()
                .is_none()
        );
    }

    #[test]
    fn text_format_yields_no_constraint() {
        assert!(
            OpenAIResponsesTextParam {
                format: Some(OpenAIResponsesTextFormat::Text),
            }
            .into_grammar_constraint()
            .is_none()
        );
    }

    #[test]
    fn unsupported_format_yields_no_constraint() {
        assert!(
            OpenAIResponsesTextParam {
                format: Some(OpenAIResponsesTextFormat::Unsupported),
            }
            .into_grammar_constraint()
            .is_none()
        );
    }

    #[test]
    fn json_schema_format_yields_a_json_schema_constraint() {
        let constraint = OpenAIResponsesTextParam {
            format: Some(OpenAIResponsesTextFormat::JsonSchema {
                schema: json!({ "type": "object" }),
            }),
        }
        .into_grammar_constraint()
        .expect("json_schema format yields a constraint");

        assert_eq!(
            constraint,
            GrammarConstraint::JsonSchema {
                schema: json!({ "type": "object" }).to_string(),
            }
        );
    }
}
