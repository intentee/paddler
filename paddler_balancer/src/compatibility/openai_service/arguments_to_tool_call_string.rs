use anyhow::Context as _;
use anyhow::Result;
use llama_cpp_bindings_types::ToolCallArguments;

pub fn arguments_to_tool_call_string(arguments: &ToolCallArguments) -> Result<String> {
    match arguments {
        ToolCallArguments::ValidJson(value) => {
            serde_json::to_string(value).context("serializing tool-call arguments to OpenAI string")
        }
        ToolCallArguments::InvalidJson(raw) => Ok(raw.clone()),
    }
}

#[cfg(test)]
mod tests {
    use llama_cpp_bindings_types::ToolCallArguments;

    use super::arguments_to_tool_call_string;

    #[test]
    fn serializes_valid_json_arguments() {
        let serialized =
            arguments_to_tool_call_string(&ToolCallArguments::ValidJson(serde_json::json!({
                "location": "Paris"
            })))
            .unwrap();

        assert_eq!(serialized, "{\"location\":\"Paris\"}");
    }

    #[test]
    fn passes_invalid_json_through_verbatim() {
        let serialized =
            arguments_to_tool_call_string(&ToolCallArguments::InvalidJson("{not valid".to_owned()))
                .unwrap();

        assert_eq!(serialized, "{not valid");
    }
}
