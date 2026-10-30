use serde_json::Value;

use crate::python_float_repr::python_float_repr;

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

#[must_use]
pub fn render_json_content(value: &Value, indent: usize) -> String {
    let pad = "  ".repeat(indent);

    match value {
        Value::Array(items) => items
            .iter()
            .map(|item| {
                format!(
                    "{pad}- {}",
                    render_json_content(item, indent + 1).trim_start_matches(python_whitespace)
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(fields) => fields
            .iter()
            .map(|(key, field)| match field {
                Value::Array(_) | Value::Object(_) => {
                    format!("{pad}{key}:\n{}", render_json_content(field, indent + 1))
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
