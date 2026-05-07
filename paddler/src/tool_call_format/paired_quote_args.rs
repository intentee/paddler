use anyhow::Result;
use anyhow::anyhow;
use llama_cpp_bindings::PairedQuoteShape;
use llama_cpp_bindings::ParsedToolCall;
use llama_cpp_bindings::ToolCallArguments;
use llama_cpp_bindings::ToolCallMarkers;
use llama_cpp_bindings::ToolCallValueQuote;

pub fn try_parse(
    body: &str,
    markers: &ToolCallMarkers,
    shape: &PairedQuoteShape,
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
        let args_body_start = &after_open[separator_position + shape.name_args_separator.len()..];

        let (arguments_text, after_arguments) =
            translate_paired_quote_args(args_body_start, &shape.value_quote, &markers.close)?;
        let arguments = ToolCallArguments::from_string(arguments_text);
        if matches!(arguments, ToolCallArguments::InvalidJson(_)) {
            return Err(anyhow!(
                "translated tool call arguments are not valid JSON for tool '{name}'"
            ));
        }

        parsed.push(ParsedToolCall::new(String::new(), name, arguments));
        remaining = after_arguments.trim_start();
    }

    Ok(parsed)
}

#[derive(Debug, Eq, PartialEq)]
enum ParserState {
    BeforeKey,
    InsideKey,
    AfterKey,
    InsideQuotedValue,
    InsideBareValue,
    AfterValue,
}

fn translate_paired_quote_args<'body>(
    input: &'body str,
    value_quote: &ToolCallValueQuote,
    close_marker: &str,
) -> Result<(String, &'body str)> {
    let mut state = ParserState::BeforeKey;
    let mut output = String::from("{");
    let mut key_buffer = String::new();
    let mut value_buffer = String::new();
    let mut byte_position = 0usize;
    let bytes = input.as_bytes();

    while byte_position < bytes.len() {
        let remaining = &input[byte_position..];

        if matches!(state, ParserState::AfterValue | ParserState::BeforeKey)
            && !close_marker.is_empty()
            && remaining.starts_with(close_marker)
        {
            output.push('}');
            byte_position += close_marker.len();
            return Ok((output, &input[byte_position..]));
        }
        if matches!(state, ParserState::InsideBareValue)
            && !close_marker.is_empty()
            && remaining.starts_with(close_marker)
        {
            push_bare_value(&mut output, value_buffer.trim());
            value_buffer.clear();
            output.push('}');
            byte_position += close_marker.len();
            return Ok((output, &input[byte_position..]));
        }

        let Some(current_char) = remaining.chars().next() else {
            break;
        };
        let char_len = current_char.len_utf8();

        match state {
            ParserState::BeforeKey => {
                if current_char.is_whitespace() {
                    byte_position += char_len;
                } else {
                    key_buffer.push(current_char);
                    byte_position += char_len;
                    state = ParserState::InsideKey;
                }
            }
            ParserState::InsideKey => {
                if current_char == ':' {
                    let key = key_buffer.trim();
                    if key.is_empty() {
                        return Err(anyhow!("empty key in tool call arguments"));
                    }
                    output.push('"');
                    push_json_escaped(&mut output, key);
                    output.push_str("\":");
                    key_buffer.clear();
                    byte_position += char_len;
                    state = ParserState::AfterKey;
                } else {
                    key_buffer.push(current_char);
                    byte_position += char_len;
                }
            }
            ParserState::AfterKey => {
                if current_char.is_whitespace() {
                    byte_position += char_len;
                } else if remaining.starts_with(value_quote.open.as_str()) {
                    byte_position += value_quote.open.len();
                    state = ParserState::InsideQuotedValue;
                } else {
                    value_buffer.push(current_char);
                    byte_position += char_len;
                    state = ParserState::InsideBareValue;
                }
            }
            ParserState::InsideQuotedValue => {
                if remaining.starts_with(value_quote.close.as_str()) {
                    output.push('"');
                    push_json_escaped(&mut output, &value_buffer);
                    output.push('"');
                    value_buffer.clear();
                    byte_position += value_quote.close.len();
                    state = ParserState::AfterValue;
                } else {
                    value_buffer.push(current_char);
                    byte_position += char_len;
                }
            }
            ParserState::InsideBareValue => {
                if current_char == ',' {
                    push_bare_value(&mut output, value_buffer.trim());
                    value_buffer.clear();
                    output.push(',');
                    byte_position += char_len;
                    state = ParserState::BeforeKey;
                } else {
                    value_buffer.push(current_char);
                    byte_position += char_len;
                }
            }
            ParserState::AfterValue => {
                if current_char.is_whitespace() {
                    byte_position += char_len;
                } else if current_char == ',' {
                    output.push(',');
                    byte_position += char_len;
                    state = ParserState::BeforeKey;
                } else {
                    return Err(anyhow!(
                        "unexpected character '{current_char}' after tool call value; \
                         expected ',' or close marker"
                    ));
                }
            }
        }
    }

    match state {
        ParserState::AfterValue | ParserState::BeforeKey => {
            output.push('}');
            Ok((output, ""))
        }
        ParserState::InsideBareValue => {
            push_bare_value(&mut output, value_buffer.trim());
            output.push('}');
            Ok((output, ""))
        }
        _ => Err(anyhow!(
            "tool call arguments ended in {state:?} state without close marker"
        )),
    }
}

fn push_bare_value(output: &mut String, value: &str) {
    if value.is_empty() {
        output.push_str("null");
    } else if serde_json::from_str::<serde_json::Value>(value).is_ok() {
        output.push_str(value);
    } else {
        output.push('"');
        push_json_escaped(output, value);
        output.push('"');
    }
}

fn push_lower_hex_byte(output: &mut String, byte: u8) {
    output.push(hex_nibble(byte >> 4));
    output.push(hex_nibble(byte & 0x0f));
}

fn hex_nibble(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'a' + (nibble - 10)),
        _ => '0',
    }
}

fn push_json_escaped(output: &mut String, raw: &str) {
    for character in raw.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            other if (other as u32) < 0x20 => {
                output.push_str("\\u00");
                push_lower_hex_byte(output, other as u8);
            }
            other => output.push(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use llama_cpp_bindings::PairedQuoteShape;
    use llama_cpp_bindings::ToolCallArgsShape;
    use llama_cpp_bindings::ToolCallMarkers;
    use llama_cpp_bindings::ToolCallValueQuote;
    use serde_json::json;

    use super::ParserState;
    use super::ToolCallArguments;
    use super::translate_paired_quote_args;
    use super::try_parse;

    fn gemma4_markers() -> ToolCallMarkers {
        ToolCallMarkers {
            open: "<|tool_call>call:".to_owned(),
            close: "}".to_owned(),
            args_shape: ToolCallArgsShape::PairedQuote(gemma4_shape()),
        }
    }

    fn gemma4_shape() -> PairedQuoteShape {
        PairedQuoteShape {
            name_args_separator: "{".to_owned(),
            value_quote: ToolCallValueQuote {
                open: "<|\"|>".to_owned(),
                close: "<|\"|>".to_owned(),
            },
        }
    }

    fn gemma4_value_quote() -> ToolCallValueQuote {
        ToolCallValueQuote {
            open: "<|\"|>".to_owned(),
            close: "<|\"|>".to_owned(),
        }
    }

    #[test]
    fn parses_single_quoted_string_argument_with_full_markers() -> Result<()> {
        let parsed = try_parse(
            "<|tool_call>call:get_weather{location:<|\"|>Paris<|\"|>}",
            &gemma4_markers(),
            &gemma4_shape(),
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
    fn parses_classifier_stripped_body_without_open_or_close() -> Result<()> {
        let parsed = try_parse(
            "get_weather{location:<|\"|>Paris<|\"|>",
            &gemma4_markers(),
            &gemma4_shape(),
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
    fn parses_multiple_quoted_string_arguments() -> Result<()> {
        let parsed = try_parse(
            "<|tool_call>call:f{a:<|\"|>1<|\"|>,b:<|\"|>2<|\"|>}",
            &gemma4_markers(),
            &gemma4_shape(),
        )?;

        assert_eq!(parsed.len(), 1);
        assert_eq!(
            parsed[0].arguments,
            ToolCallArguments::ValidJson(json!({"a": "1", "b": "2"})),
        );
        Ok(())
    }

    #[test]
    fn parses_bare_numeric_value() -> Result<()> {
        let parsed = try_parse(
            "<|tool_call>call:f{a:42}",
            &gemma4_markers(),
            &gemma4_shape(),
        )?;

        assert_eq!(
            parsed[0].arguments,
            ToolCallArguments::ValidJson(json!({"a": 42})),
        );
        Ok(())
    }

    #[test]
    fn parses_bare_boolean_value() -> Result<()> {
        let parsed = try_parse(
            "<|tool_call>call:f{a:true}",
            &gemma4_markers(),
            &gemma4_shape(),
        )?;

        assert_eq!(
            parsed[0].arguments,
            ToolCallArguments::ValidJson(json!({"a": true})),
        );
        Ok(())
    }

    #[test]
    fn rejects_unclosed_quoted_value() {
        let result = try_parse(
            "<|tool_call>call:f{a:<|\"|>oops",
            &gemma4_markers(),
            &gemma4_shape(),
        );

        assert!(result.is_err(), "unclosed quote must fail; got {result:?}");
    }

    #[test]
    fn returns_empty_vec_for_empty_body() -> Result<()> {
        let parsed = try_parse("", &gemma4_markers(), &gemma4_shape())?;
        assert!(parsed.is_empty());
        Ok(())
    }

    #[test]
    fn returns_empty_vec_when_body_lacks_separator() -> Result<()> {
        let parsed = try_parse("no separator anywhere", &gemma4_markers(), &gemma4_shape())?;
        assert!(parsed.is_empty());
        Ok(())
    }

    #[test]
    fn state_before_key_consumes_whitespace_then_starts_key() -> Result<()> {
        let (translated, _) =
            translate_paired_quote_args("  alpha:<|\"|>v<|\"|>}", &gemma4_value_quote(), "}")?;

        assert_eq!(translated, "{\"alpha\":\"v\"}");
        Ok(())
    }

    #[test]
    fn state_inside_bare_value_terminated_by_close_marker() -> Result<()> {
        let (translated, rest) =
            translate_paired_quote_args("n:1}leftover", &gemma4_value_quote(), "}")?;

        assert_eq!(translated, "{\"n\":1}");
        assert_eq!(rest, "leftover");
        Ok(())
    }

    #[test]
    fn state_after_value_followed_by_comma_starts_next_key() -> Result<()> {
        let (translated, _) = translate_paired_quote_args(
            "x:<|\"|>a<|\"|>,y:<|\"|>b<|\"|>}",
            &gemma4_value_quote(),
            "}",
        )?;

        assert_eq!(translated, "{\"x\":\"a\",\"y\":\"b\"}");
        Ok(())
    }

    #[test]
    fn state_after_value_with_unexpected_char_returns_err() {
        let result =
            translate_paired_quote_args("x:<|\"|>v<|\"|>$bad}", &gemma4_value_quote(), "}");

        assert!(
            result.is_err(),
            "garbage after value must fail; got {result:?}"
        );
    }

    #[test]
    fn translator_terminates_on_end_of_input_after_quoted_value() -> Result<()> {
        let (translated, rest) =
            translate_paired_quote_args("x:<|\"|>v<|\"|>", &gemma4_value_quote(), "}")?;

        assert_eq!(translated, "{\"x\":\"v\"}");
        assert_eq!(rest, "");
        Ok(())
    }

    #[test]
    fn translator_terminates_on_end_of_input_after_bare_value() -> Result<()> {
        let (translated, rest) = translate_paired_quote_args("n:42", &gemma4_value_quote(), "}")?;

        assert_eq!(translated, "{\"n\":42}");
        assert_eq!(rest, "");
        Ok(())
    }

    #[test]
    fn parser_state_variants_are_distinct() {
        let all = [
            ParserState::BeforeKey,
            ParserState::InsideKey,
            ParserState::AfterKey,
            ParserState::InsideQuotedValue,
            ParserState::InsideBareValue,
            ParserState::AfterValue,
        ];
        for (index, state) in all.iter().enumerate() {
            for (other_index, other) in all.iter().enumerate() {
                if index == other_index {
                    assert_eq!(state, other);
                } else {
                    assert_ne!(state, other);
                }
            }
        }
    }
}
