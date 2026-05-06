use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::ToolCallArguments;
use llama_cpp_bindings::ToolCallMarkers;
use llama_cpp_bindings::XmlTagsShape;
use serde_json::Map;
use serde_json::Value;

pub fn try_parse(
    body: &str,
    _markers: &ToolCallMarkers,
    shape: &XmlTagsShape,
) -> Result<Vec<ParsedToolCall>> {
    if shape.function_open_prefix.is_empty()
        || shape.function_close.is_empty()
        || shape.parameter_open_prefix.is_empty()
        || shape.parameter_close.is_empty()
    {
        return Ok(Vec::new());
    }

    let mut parsed = Vec::new();
    let mut remaining = body;

    while let Some(function_start) = remaining.find(shape.function_open_prefix.as_str()) {
        let after_function_prefix =
            &remaining[function_start + shape.function_open_prefix.len()..];
        let name_end = bounded_tag_name_end(
            after_function_prefix,
            &shape.function_open_prefix,
        )?;
        let function_name = after_function_prefix[..name_end].trim().to_owned();
        if function_name.is_empty() {
            return Err(anyhow!("tool call function tag has empty name"));
        }
        let function_body_start = &after_function_prefix[name_end + 1..];

        let Some(function_body_end_relative) =
            function_body_start.find(shape.function_close.as_str())
        else {
            return Err(anyhow!(
                "tool call function block for '{}' is missing close tag '{}'",
                function_name,
                shape.function_close,
            ));
        };
        let function_body = &function_body_start[..function_body_end_relative];
        let after_function_close =
            &function_body_start[function_body_end_relative + shape.function_close.len()..];

        let arguments_object = collect_parameters(function_body, shape)?;
        let arguments_value = Value::Object(arguments_object);
        let arguments = ToolCallArguments::from_string(arguments_value.to_string());

        parsed.push(ParsedToolCall::new(String::new(), function_name, arguments));
        remaining = after_function_close;
    }

    Ok(parsed)
}

fn collect_parameters(
    function_body: &str,
    shape: &XmlTagsShape,
) -> Result<Map<String, Value>> {
    let mut arguments = Map::new();
    let mut remaining = function_body;

    while let Some(parameter_start) = remaining.find(shape.parameter_open_prefix.as_str()) {
        let after_parameter_prefix =
            &remaining[parameter_start + shape.parameter_open_prefix.len()..];
        let name_end = bounded_tag_name_end(
            after_parameter_prefix,
            &shape.parameter_open_prefix,
        )?;
        let parameter_name = after_parameter_prefix[..name_end].trim().to_owned();
        if parameter_name.is_empty() {
            return Err(anyhow!("tool call parameter tag has empty name"));
        }
        let value_start = &after_parameter_prefix[name_end + 1..];

        let Some(value_end_relative) = value_start.find(shape.parameter_close.as_str()) else {
            return Err(anyhow!(
                "tool call parameter '{}' is missing close tag '{}'",
                parameter_name,
                shape.parameter_close,
            ));
        };
        let raw_value = trim_surrounding_newlines(&value_start[..value_end_relative]);
        let after_parameter_close =
            &value_start[value_end_relative + shape.parameter_close.len()..];

        arguments.insert(parameter_name, parse_parameter_value(raw_value));
        remaining = after_parameter_close;
    }

    Ok(arguments)
}

fn trim_surrounding_newlines(input: &str) -> &str {
    input.trim_start_matches('\n').trim_end_matches('\n')
}

fn bounded_tag_name_end(after_prefix: &str, opening_prefix: &str) -> Result<usize> {
    let close_position = after_prefix.find('>');
    let next_open_position = after_prefix.find('<');
    match (close_position, next_open_position) {
        (Some(close), Some(open)) if open < close => Err(anyhow!(
            "tool call tag opened by '{opening_prefix}' is missing closing '>' before next '<'"
        )),
        (Some(close), _) => Ok(close),
        (None, _) => Err(anyhow!(
            "tool call tag opened by '{opening_prefix}' is missing closing '>'"
        )),
    }
}

fn parse_parameter_value(raw: &str) -> Value {
    match serde_json::from_str::<Value>(raw) {
        Ok(value) => value,
        Err(_not_json) => Value::String(raw.to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use llama_cpp_bindings::ToolCallArgsShape;
    use llama_cpp_bindings::ToolCallMarkers;
    use llama_cpp_bindings::XmlTagsShape;
    use serde_json::json;

    use super::ToolCallArguments;
    use super::try_parse;

    fn xml_shape() -> XmlTagsShape {
        XmlTagsShape {
            function_open_prefix: "<function=".to_owned(),
            function_close: "</function>".to_owned(),
            parameter_open_prefix: "<parameter=".to_owned(),
            parameter_close: "</parameter>".to_owned(),
        }
    }

    fn xml_markers() -> ToolCallMarkers {
        ToolCallMarkers {
            open: "<tool_call>".to_owned(),
            close: "</tool_call>".to_owned(),
            args_shape: ToolCallArgsShape::XmlTags(xml_shape()),
        }
    }

    #[test]
    fn parses_single_function_with_one_parameter() -> Result<()> {
        let body = "\n<function=get_weather>\n<parameter=location>\nParis\n</parameter>\n</function>\n";
        let parsed = try_parse(body, &xml_markers(), &xml_shape())?;

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "get_weather");
        assert_eq!(
            parsed[0].arguments,
            ToolCallArguments::ValidJson(json!({"location": "Paris"})),
        );
        Ok(())
    }

    #[test]
    fn parses_function_with_multiple_parameters() -> Result<()> {
        let body = "<function=f><parameter=a>1</parameter><parameter=b>two</parameter></function>";
        let parsed = try_parse(body, &xml_markers(), &xml_shape())?;

        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0].arguments,
            ToolCallArguments::ValidJson(json!({"a": 1, "b": "two"})),
        );
        Ok(())
    }

    #[test]
    fn parses_two_function_blocks_in_one_body() -> Result<()> {
        let body = "<function=a><parameter=x>1</parameter></function><function=b><parameter=y>2</parameter></function>";
        let parsed = try_parse(body, &xml_markers(), &xml_shape())?;

        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "a");
        assert_eq!(parsed[1].name, "b");
        Ok(())
    }

    #[test]
    fn preserves_multi_line_parameter_value() -> Result<()> {
        let body = "<function=f>\n<parameter=msg>\nline one\nline two\n</parameter>\n</function>";
        let parsed = try_parse(body, &xml_markers(), &xml_shape())?;

        assert_eq!(
            parsed[0].arguments,
            ToolCallArguments::ValidJson(json!({"msg": "line one\nline two"})),
        );
        Ok(())
    }

    #[test]
    fn rejects_function_tag_missing_closing_angle() {
        let body = "<function=get_weather\n<parameter=location>Paris</parameter></function>";
        let result = try_parse(body, &xml_markers(), &xml_shape());

        assert!(
            result.is_err(),
            "function tag missing '>' must produce Err, got {result:?}",
        );
    }

    #[test]
    fn rejects_function_block_missing_close_tag() {
        let body = "<function=get_weather><parameter=location>Paris</parameter>";
        let result = try_parse(body, &xml_markers(), &xml_shape());

        assert!(
            result.is_err(),
            "function block without close tag must produce Err, got {result:?}",
        );
    }

    #[test]
    fn rejects_parameter_block_missing_close_tag() {
        let body = "<function=get_weather><parameter=location>Paris</function>";
        let result = try_parse(body, &xml_markers(), &xml_shape());

        assert!(
            result.is_err(),
            "parameter block without close tag must produce Err, got {result:?}",
        );
    }

    #[test]
    fn returns_empty_when_body_has_no_function_tag() -> Result<()> {
        let body = "plain text without function tags";
        let parsed = try_parse(body, &xml_markers(), &xml_shape())?;
        assert!(parsed.is_empty());
        Ok(())
    }

    #[test]
    fn returns_empty_for_empty_body() -> Result<()> {
        let parsed = try_parse("", &xml_markers(), &xml_shape())?;
        assert!(parsed.is_empty());
        Ok(())
    }

    #[test]
    fn returns_empty_when_shape_has_empty_required_field() -> Result<()> {
        let mut shape = xml_shape();
        shape.function_close.clear();
        let body = "<function=f><parameter=x>1</parameter></function>";
        let parsed = try_parse(body, &xml_markers(), &shape)?;
        assert!(parsed.is_empty());
        Ok(())
    }
}
