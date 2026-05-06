pub mod bracketed_args;
pub mod paired_quote_args;
pub mod xml_function_tags;

use anyhow::Result;
use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::ToolCallArgsShape;
use llama_cpp_bindings::ToolCallMarkers;

pub fn try_parse(body: &str, markers: &ToolCallMarkers) -> Result<Vec<ParsedToolCall>> {
    if markers.open.is_empty() {
        return Ok(Vec::new());
    }
    match &markers.args_shape {
        ToolCallArgsShape::BracketedJson(shape) => bracketed_args::try_parse(body, markers, shape),
        ToolCallArgsShape::PairedQuote(shape) => paired_quote_args::try_parse(body, markers, shape),
        ToolCallArgsShape::XmlTags(shape) => xml_function_tags::try_parse(body, markers, shape),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use llama_cpp_bindings::BracketedJsonShape;
    use llama_cpp_bindings::PairedQuoteShape;
    use llama_cpp_bindings::ToolCallArgsShape;
    use llama_cpp_bindings::ToolCallArguments;
    use llama_cpp_bindings::ToolCallMarkers;
    use llama_cpp_bindings::ToolCallValueQuote;
    use serde_json::json;

    use super::try_parse;

    fn mistral3_markers() -> ToolCallMarkers {
        ToolCallMarkers {
            open: "[TOOL_CALLS]".to_owned(),
            close: String::new(),
            args_shape: ToolCallArgsShape::BracketedJson(BracketedJsonShape {
                name_args_separator: "[ARGS]".to_owned(),
            }),
        }
    }

    fn gemma4_markers() -> ToolCallMarkers {
        ToolCallMarkers {
            open: "<|tool_call>call:".to_owned(),
            close: "}".to_owned(),
            args_shape: ToolCallArgsShape::PairedQuote(PairedQuoteShape {
                name_args_separator: "{".to_owned(),
                value_quote: ToolCallValueQuote {
                    open: "<|\"|>".to_owned(),
                    close: "<|\"|>".to_owned(),
                },
            }),
        }
    }

    #[test]
    fn dispatches_to_bracketed_args_for_mistral3_shape() -> Result<()> {
        let parsed = try_parse(
            "[TOOL_CALLS]get_weather[ARGS]{\"location\":\"Paris\"}",
            &mistral3_markers(),
        )?;

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "get_weather");
        assert_eq!(
            parsed[0].arguments,
            ToolCallArguments::ValidJson(json!({"location": "Paris"})),
        );
        Ok(())
    }

    #[test]
    fn dispatches_to_paired_quote_args_for_gemma4_shape() -> Result<()> {
        let parsed = try_parse(
            "<|tool_call>call:get_weather{location:<|\"|>Paris<|\"|>}",
            &gemma4_markers(),
        )?;

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "get_weather");
        assert_eq!(
            parsed[0].arguments,
            ToolCallArguments::ValidJson(json!({"location": "Paris"})),
        );
        Ok(())
    }

    #[test]
    fn returns_empty_for_markers_with_empty_open() -> Result<()> {
        let markers = ToolCallMarkers {
            open: String::new(),
            close: String::new(),
            args_shape: ToolCallArgsShape::BracketedJson(BracketedJsonShape {
                name_args_separator: "[ARGS]".to_owned(),
            }),
        };
        let parsed = try_parse("[TOOL_CALLS]get_weather[ARGS]{}", &markers)?;
        assert!(parsed.is_empty());
        Ok(())
    }
}
