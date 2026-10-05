use std::collections::BTreeMap;
use std::collections::BTreeSet;

use anyhow::Context as _;
use anyhow::Result;
use anyhow::bail;
use serde_json::Map;
use serde_json::Value;
use serde_json::json;

const COMPONENT_REF_PREFIX: &str = "#/components/schemas/";
const DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

fn rewrite_ref(reference: &Value) -> Value {
    match reference {
        Value::String(reference) => reference.strip_prefix(COMPONENT_REF_PREFIX).map_or_else(
            || Value::String(reference.clone()),
            |name| Value::String(format!("#/$defs/{name}")),
        ),
        other => transform_node(other),
    }
}

fn unique_strings(values: &[Value]) -> Vec<Value> {
    let mut seen = BTreeSet::new();
    let mut unique = Vec::new();

    for value in values {
        let key = value.to_string();

        if seen.insert(key) {
            unique.push(value.clone());
        }
    }

    unique
}

fn transform_object(object: &Map<String, Value>) -> Value {
    let mut transformed = Map::new();
    let mut nullable = false;

    for (key, value) in object {
        match key.as_str() {
            "nullable" => nullable = matches!(value, Value::Bool(true)),
            // Draft 2019-09 recursion keywords the OpenAI document still carries; Draft 2020-12
            // replaced them with `$dynamicAnchor`/`$dynamicRef`. Drop them so the assembled schema
            // passes 2020-12 meta-validation. The schemas that use them (recursive filters) are not
            // part of any Paddler-emitted payload, so removing the recursion is inconsequential.
            "$recursiveAnchor" | "$recursiveRef" => {}
            // OpenAPI 3.0 expressed exclusive bounds as booleans; Draft 2020-12 expects the bound to
            // be the number itself. A boolean form is meaningless under 2020-12, so drop it.
            "exclusiveMinimum" | "exclusiveMaximum" if value.is_boolean() => {}
            "required" => {
                if let Value::Array(entries) = value {
                    transformed.insert(key.clone(), Value::Array(unique_strings(entries)));
                } else {
                    transformed.insert(key.clone(), transform_node(value));
                }
            }
            "$ref" => {
                transformed.insert("$ref".to_owned(), rewrite_ref(value));
            }
            _ => {
                transformed.insert(key.clone(), transform_node(value));
            }
        }
    }

    if nullable {
        let mut nullable_wrapper = Map::new();
        nullable_wrapper.insert(
            "anyOf".to_owned(),
            Value::Array(vec![Value::Object(transformed), json!({ "type": "null" })]),
        );

        Value::Object(nullable_wrapper)
    } else {
        Value::Object(transformed)
    }
}

fn transform_node(node: &Value) -> Value {
    match node {
        Value::Array(items) => Value::Array(items.iter().map(transform_node).collect()),
        Value::Object(object) => transform_object(object),
        other => other.clone(),
    }
}

fn collect_component_refs(node: &Value, found: &mut BTreeSet<String>) {
    match node {
        Value::Array(items) => {
            for item in items {
                collect_component_refs(item, found);
            }
        }
        Value::Object(object) => {
            for (key, value) in object {
                if key == "$ref"
                    && let Value::String(reference) = value
                    && let Some(name) = reference.strip_prefix(COMPONENT_REF_PREFIX)
                {
                    found.insert(name.to_owned());
                } else {
                    collect_component_refs(value, found);
                }
            }
        }
        _ => {}
    }
}

fn transitive_closure<'spec>(
    components: &'spec Value,
    root_name: &str,
) -> Result<BTreeMap<String, &'spec Value>> {
    let mut reachable: BTreeMap<String, &'spec Value> = BTreeMap::new();
    let mut pending = vec![root_name.to_owned()];

    while let Some(name) = pending.pop() {
        if reachable.contains_key(&name) {
            continue;
        }

        let component = components
            .get(name.as_str())
            .with_context(|| format!("schema references unknown component {name:?}"))?;

        reachable.insert(name.clone(), component);

        let mut direct_refs = BTreeSet::new();
        collect_component_refs(component, &mut direct_refs);

        for reference in direct_refs {
            pending.push(reference);
        }
    }

    Ok(reachable)
}

pub fn strict_chat_completion_schema(
    components: &Value,
    root_name: &str,
    strict_pointers: &[&str],
) -> Result<Value> {
    let closure = transitive_closure(components, root_name)?;

    let mut definitions = Map::new();

    for (name, component) in closure {
        definitions.insert(name, transform_node(component));
    }

    let mut schema = json!({
        "$schema": DIALECT,
        "$ref": format!("#/$defs/{root_name}"),
        "$defs": Value::Object(definitions),
    });

    for pointer in strict_pointers {
        match schema.pointer_mut(pointer) {
            Some(Value::Object(target)) => {
                target.insert("unevaluatedProperties".to_owned(), Value::Bool(false));
            }
            Some(other) => bail!("strict target {pointer:?} is not an object: {other}"),
            None => bail!("strict target {pointer:?} was not found in the assembled schema"),
        }
    }

    Ok(schema)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::strict_chat_completion_schema;
    use super::transform_node;

    #[test]
    fn rewrites_component_refs_to_defs() {
        let rewritten = transform_node(&json!({ "$ref": "#/components/schemas/Foo" }));

        assert_eq!(rewritten, json!({ "$ref": "#/$defs/Foo" }));
    }

    #[test]
    fn leaves_non_component_refs_untouched() {
        let rewritten = transform_node(&json!({ "$ref": "#/$defs/Foo" }));

        assert_eq!(rewritten, json!({ "$ref": "#/$defs/Foo" }));
    }

    #[test]
    fn leaves_non_string_refs_untouched() {
        let rewritten = transform_node(&json!({ "$ref": 42 }));

        assert_eq!(rewritten, json!({ "$ref": 42 }));
    }

    #[test]
    fn wraps_nullable_into_anyof_null() {
        let wrapped = transform_node(&json!({ "type": "string", "nullable": true }));

        assert_eq!(
            wrapped,
            json!({ "anyOf": [{ "type": "string" }, { "type": "null" }] })
        );
    }

    #[test]
    fn drops_nullable_false() {
        let transformed = transform_node(&json!({ "type": "string", "nullable": false }));

        assert_eq!(transformed, json!({ "type": "string" }));
    }

    #[test]
    fn drops_draft_2019_recursive_keywords() {
        let transformed = transform_node(&json!({
            "$recursiveAnchor": true,
            "$recursiveRef": "#",
            "type": "object"
        }));

        assert_eq!(transformed, json!({ "type": "object" }));
    }

    #[test]
    fn drops_boolean_exclusive_bounds() {
        let transformed = transform_node(&json!({
            "type": "number",
            "minimum": 0,
            "exclusiveMinimum": true
        }));

        assert_eq!(transformed, json!({ "type": "number", "minimum": 0 }));
    }

    #[test]
    fn keeps_numeric_exclusive_bounds() {
        let transformed = transform_node(&json!({ "type": "number", "exclusiveMinimum": 0 }));

        assert_eq!(
            transformed,
            json!({ "type": "number", "exclusiveMinimum": 0 })
        );
    }

    #[test]
    fn deduplicates_required_entries() {
        let transformed = transform_node(&json!({
            "type": "object",
            "required": ["id", "name", "id"]
        }));

        assert_eq!(
            transformed,
            json!({ "type": "object", "required": ["id", "name"] })
        );
    }

    #[test]
    fn passes_a_non_array_required_through_untouched() {
        let transformed = transform_node(&json!({ "type": "object", "required": "id" }));

        assert_eq!(transformed, json!({ "type": "object", "required": "id" }));
    }

    #[test]
    fn passes_scalars_through() {
        assert_eq!(transform_node(&json!(7)), json!(7));
    }

    #[test]
    fn transforms_refs_nested_in_arrays() {
        let transformed =
            transform_node(&json!({ "allOf": [{ "$ref": "#/components/schemas/Sub" }] }));

        assert_eq!(transformed, json!({ "allOf": [{ "$ref": "#/$defs/Sub" }] }));
    }

    #[test]
    fn builds_self_contained_schema_with_strict_targets() {
        let components = json!({
            "Root": {
                "type": "object",
                "properties": { "child": { "$ref": "#/components/schemas/Child" } }
            },
            "Child": { "type": "object" }
        });

        let schema =
            strict_chat_completion_schema(&components, "Root", &["/$defs/Root", "/$defs/Child"])
                .unwrap();

        assert_eq!(schema["$ref"], json!("#/$defs/Root"));
        assert_eq!(
            schema["$defs"]["Root"]["unevaluatedProperties"],
            json!(false)
        );
        assert_eq!(
            schema["$defs"]["Child"]["unevaluatedProperties"],
            json!(false)
        );
    }

    #[test]
    fn collects_a_shared_component_reached_through_two_paths_only_once() {
        let components = json!({
            "Root": {
                "type": "object",
                "properties": {
                    "left": { "$ref": "#/components/schemas/Left" },
                    "right": { "$ref": "#/components/schemas/Right" }
                }
            },
            "Left": { "properties": { "shared": { "$ref": "#/components/schemas/Shared" } } },
            "Right": { "properties": { "shared": { "$ref": "#/components/schemas/Shared" } } },
            "Shared": { "type": "object" }
        });

        let schema = strict_chat_completion_schema(&components, "Root", &[]).unwrap();

        assert!(schema["$defs"]["Shared"].is_object());
        assert_eq!(schema["$defs"].as_object().unwrap().len(), 4);
    }

    #[test]
    fn rejects_unknown_root_schema() {
        let error = strict_chat_completion_schema(&json!({}), "Root", &[]).unwrap_err();

        assert!(error.to_string().contains("unknown component \"Root\""));
    }

    #[test]
    fn rejects_dangling_component_ref() {
        let components = json!({ "Root": { "$ref": "#/components/schemas/Missing" } });

        let error = strict_chat_completion_schema(&components, "Root", &[]).unwrap_err();

        assert!(error.to_string().contains("unknown component \"Missing\""));
    }

    #[test]
    fn rejects_missing_strict_target() {
        let components = json!({ "Root": { "type": "object" } });

        let error =
            strict_chat_completion_schema(&components, "Root", &["/$defs/Nope"]).unwrap_err();

        assert!(error.to_string().contains("was not found"));
    }

    #[test]
    fn rejects_non_object_strict_target() {
        let components = json!({ "Root": { "type": "object" } });

        let error = strict_chat_completion_schema(&components, "Root", &["/$ref"]).unwrap_err();

        assert!(error.to_string().contains("is not an object"));
    }
}
