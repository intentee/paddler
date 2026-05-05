use serde::Deserialize;
use serde::Serialize;

/// Wire-format value object for one parsed tool call.
///
/// Mirrors the bindings' `llama_cpp_bindings::ParsedToolCall` so it can be
/// serialised straight into both Paddler-native SSE streams and OpenAI-compat
/// `delta.tool_calls` chunks. The `arguments_json` field is the raw JSON
/// string emitted by the parser; downstream consumers should validate it
/// against the tool's parameter schema before acting on it.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ParsedToolCall {
    pub id: String,
    pub name: String,
    pub arguments_json: String,
}

impl ParsedToolCall {
    #[must_use]
    pub const fn new(id: String, name: String, arguments_json: String) -> Self {
        Self {
            id,
            name,
            arguments_json,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ParsedToolCall;

    #[test]
    fn new_assigns_fields_in_order() {
        let parsed = ParsedToolCall::new(
            "id-1".to_owned(),
            "tool".to_owned(),
            "{}".to_owned(),
        );

        assert_eq!(parsed.id, "id-1");
        assert_eq!(parsed.name, "tool");
        assert_eq!(parsed.arguments_json, "{}");
    }

    #[test]
    fn default_is_empty_strings() {
        let parsed = ParsedToolCall::default();

        assert!(parsed.id.is_empty());
        assert!(parsed.name.is_empty());
        assert!(parsed.arguments_json.is_empty());
    }

    #[test]
    fn rejects_unknown_fields_during_deserialization() {
        let json =
            "{\"id\":\"x\",\"name\":\"y\",\"arguments_json\":\"{}\",\"extra\":\"nope\"}";
        let result = serde_json::from_str::<ParsedToolCall>(json);

        assert!(result.is_err());
    }
}
