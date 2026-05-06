use anyhow::Context as _;
use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::BracketedJsonShape;
use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::ToolCallArguments;
use llama_cpp_bindings::ToolCallMarkers;

pub fn try_parse(
    body: &str,
    markers: &ToolCallMarkers,
    shape: &BracketedJsonShape,
) -> Result<Vec<ParsedToolCall>> {
    if shape.name_args_separator.is_empty() {
        return Ok(Vec::new());
    }

    let mut parsed = Vec::new();
    let mut remaining = body.trim_start();

    while !remaining.is_empty() {
        let after_open = remaining
            .strip_prefix(markers.open.as_str())
            .unwrap_or(remaining);

        let Some(separator_position) = after_open.find(shape.name_args_separator.as_str()) else {
            break;
        };

        let name = after_open[..separator_position].trim().to_owned();
        if name.is_empty() {
            break;
        }
        let after_separator = &after_open[separator_position + shape.name_args_separator.len()..];

        let (arguments_text, after_arguments) = consume_json_value_prefix(after_separator)?;
        let arguments = ToolCallArguments::from_string(arguments_text);
        if matches!(arguments, ToolCallArguments::InvalidJson(_)) {
            return Err(anyhow!(
                "tool call arguments are not valid JSON for tool '{name}'"
            ));
        }

        parsed.push(ParsedToolCall::new(String::new(), name, arguments));

        let after_close = if markers.close.is_empty() {
            after_arguments
        } else {
            after_arguments
                .strip_prefix(markers.close.as_str())
                .unwrap_or(after_arguments)
        };
        remaining = after_close.trim_start();
    }

    Ok(parsed)
}

fn consume_json_value_prefix(input: &str) -> Result<(String, &str)> {
    let mut stream = serde_json::Deserializer::from_str(input).into_iter::<serde_json::Value>();
    let _value = stream
        .next()
        .ok_or_else(|| anyhow!("expected a JSON value where tool call arguments start"))?
        .context("failed to parse JSON value at tool call arguments")?;
    let consumed = stream.byte_offset();
    let value_text = input[..consumed].to_owned();
    let remaining = &input[consumed..];
    Ok((value_text, remaining))
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use llama_cpp_bindings::ToolCallArgsShape;
    use llama_cpp_bindings::ToolCallMarkers;
    use serde_json::json;

    use super::BracketedJsonShape;
    use super::ToolCallArguments;
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

    fn mistral3_shape() -> BracketedJsonShape {
        BracketedJsonShape {
            name_args_separator: "[ARGS]".to_owned(),
        }
    }

    #[test]
    fn parses_single_tool_call_with_open_marker_present() -> Result<()> {
        let parsed = try_parse(
            "[TOOL_CALLS]get_weather[ARGS]{\"location\":\"Paris\"}",
            &mistral3_markers(),
            &mistral3_shape(),
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
    fn parses_single_tool_call_when_classifier_stripped_open_marker() -> Result<()> {
        let parsed = try_parse(
            "get_weather[ARGS]{\"location\":\"Paris\"}",
            &mistral3_markers(),
            &mistral3_shape(),
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
    fn parses_two_consecutive_tool_calls_with_repeated_open_marker() -> Result<()> {
        let parsed = try_parse(
            "[TOOL_CALLS]a[ARGS]{\"x\":1}[TOOL_CALLS]b[ARGS]{\"y\":2}",
            &mistral3_markers(),
            &mistral3_shape(),
        )?;

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "a");
        assert_eq!(parsed[0].arguments, ToolCallArguments::ValidJson(json!({"x": 1})));
        assert_eq!(parsed[1].name, "b");
        assert_eq!(parsed[1].arguments, ToolCallArguments::ValidJson(json!({"y": 2})));
        Ok(())
    }

    #[test]
    fn rejects_malformed_json_arguments() {
        let result = try_parse(
            "[TOOL_CALLS]get_weather[ARGS]{\"location\":}",
            &mistral3_markers(),
            &mistral3_shape(),
        );

        assert!(result.is_err(), "malformed JSON must produce Err, got {result:?}");
    }

    #[test]
    fn returns_empty_vec_for_empty_body() -> Result<()> {
        let parsed = try_parse("", &mistral3_markers(), &mistral3_shape())?;
        assert!(parsed.is_empty());
        Ok(())
    }

    #[test]
    fn returns_empty_vec_when_body_lacks_separator() -> Result<()> {
        let parsed = try_parse(
            "plain text without separator",
            &mistral3_markers(),
            &mistral3_shape(),
        )?;
        assert!(parsed.is_empty());
        Ok(())
    }
}
