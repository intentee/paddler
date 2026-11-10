use serde_json::Value;

use crate::python_float_repr::python_float_repr;

const INDENT_UNIT: &str = "  ";

fn python_whitespace(character: char) -> bool {
    character.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&character)
}

fn python_scalar(value: &Value) -> String {
    match value {
        Value::Bool(true) => "True".to_owned(),
        Value::Bool(false) => "False".to_owned(),
        Value::Number(number) => number
            .as_f64()
            .filter(|_| number.is_f64())
            .map_or_else(|| number.to_string(), python_float_repr),
        Value::String(text) => text.clone(),
        Value::Array(_) | Value::Null | Value::Object(_) => String::new(),
    }
}

fn padded_json_content(value: &Value, pad: &str) -> String {
    let nested_pad = format!("{pad}{INDENT_UNIT}");

    match value {
        Value::Array(items) => items
            .iter()
            .map(|item| {
                format!(
                    "{pad}- {}",
                    padded_json_content(item, &nested_pad).trim_start_matches(python_whitespace)
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, field)| match field {
                Value::Array(_) | Value::Object(_) => {
                    format!("{pad}{key}:\n{}", padded_json_content(field, &nested_pad))
                }
                Value::Bool(_) | Value::Null | Value::Number(_) | Value::String(_) => {
                    format!("{pad}{key}: {}", python_scalar(field))
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Bool(_) | Value::Null | Value::Number(_) | Value::String(_) => python_scalar(value),
    }
}

#[must_use]
pub fn render_json_content(value: &Value, indent: usize) -> String {
    padded_json_content(value, &INDENT_UNIT.repeat(indent))
}
