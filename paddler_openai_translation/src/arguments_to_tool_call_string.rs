use paddler_messaging::tool_call_arguments::ToolCallArguments;

#[must_use]
pub fn arguments_to_tool_call_string(arguments: &ToolCallArguments) -> String {
    match arguments {
        ToolCallArguments::ValidJson(value) => value.to_string(),
        ToolCallArguments::InvalidJson { raw, .. } => raw.clone(),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use paddler_messaging::invalid_arguments_json::InvalidArgumentsJson;
    use paddler_messaging::tool_call_arguments::ToolCallArguments;

    use super::arguments_to_tool_call_string;

    #[test]
    fn serializes_valid_json_arguments() {
        let serialized = arguments_to_tool_call_string(&ToolCallArguments::ValidJson(json!({
            "location": "Paris"
        })));

        assert_eq!(serialized, "{\"location\":\"Paris\"}");
    }

    #[test]
    fn passes_invalid_json_through_verbatim() {
        let serialized = arguments_to_tool_call_string(&ToolCallArguments::InvalidJson {
            raw: "{not valid".to_owned(),
            error: InvalidArgumentsJson {
                message: "key must be a string at line 1 column 2".to_owned(),
                line: 1,
                column: 2,
            },
        });

        assert_eq!(serialized, "{not valid");
    }
}
