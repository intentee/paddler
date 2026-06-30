use llama_cpp_bindings_types::ToolCallArguments;

#[must_use]
pub fn arguments_to_tool_call_string(arguments: &ToolCallArguments) -> String {
    match arguments {
        ToolCallArguments::ValidJson(value) => value.to_string(),
        ToolCallArguments::InvalidJson(raw) => raw.clone(),
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::ToolCallArguments;
    use serde_json::json;

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
        let serialized =
            arguments_to_tool_call_string(&ToolCallArguments::InvalidJson("{not valid".to_owned()));

        assert_eq!(serialized, "{not valid");
    }
}
