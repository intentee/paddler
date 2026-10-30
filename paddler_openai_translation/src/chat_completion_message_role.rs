use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatCompletionMessageRole {
    Assistant,
    Developer,
    System,
    Tool,
    User,
}

impl Display for ChatCompletionMessageRole {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> FmtResult {
        formatter.write_str(match self {
            Self::Assistant => "assistant",
            Self::Developer => "developer",
            Self::System => "system",
            Self::Tool => "tool",
            Self::User => "user",
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use serde_json::from_value;

    use super::ChatCompletionMessageRole;

    #[test]
    fn names_every_role_the_way_openai_requests_spell_it() {
        for role in [
            ChatCompletionMessageRole::Assistant,
            ChatCompletionMessageRole::Developer,
            ChatCompletionMessageRole::System,
            ChatCompletionMessageRole::Tool,
            ChatCompletionMessageRole::User,
        ] {
            assert_eq!(
                from_value::<ChatCompletionMessageRole>(Value::String(role.to_string())).unwrap(),
                role
            );
        }
    }
}
