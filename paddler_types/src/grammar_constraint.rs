use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
#[serde(tag = "type")]
pub enum GrammarConstraint {
    Gbnf { grammar: String, root: String },
    JsonSchema { schema: String },
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn gbnf_variant_serializes_with_type_tag() -> Result<()> {
        let constraint = GrammarConstraint::Gbnf {
            grammar: "root ::= \"yes\" | \"no\"".to_owned(),
            root: "root".to_owned(),
        };

        let json = serde_json::to_value(&constraint)?;

        assert_eq!(json["type"], "Gbnf");
        assert_eq!(json["grammar"], "root ::= \"yes\" | \"no\"");
        assert_eq!(json["root"], "root");

        Ok(())
    }

    #[test]
    fn json_schema_variant_serializes_with_type_tag() -> Result<()> {
        let constraint = GrammarConstraint::JsonSchema {
            schema: r#"{"type": "object"}"#.to_owned(),
        };

        let json = serde_json::to_value(&constraint)?;

        assert_eq!(json["type"], "JsonSchema");
        assert_eq!(json["schema"], r#"{"type": "object"}"#);

        Ok(())
    }

    #[test]
    fn gbnf_variant_round_trips() -> Result<()> {
        let constraint = GrammarConstraint::Gbnf {
            grammar: "root ::= [a-z]+".to_owned(),
            root: "root".to_owned(),
        };

        let json = serde_json::to_string(&constraint)?;
        let deserialized: GrammarConstraint = serde_json::from_str(&json)?;

        assert_eq!(constraint, deserialized);

        Ok(())
    }

    #[test]
    fn json_schema_variant_round_trips() -> Result<()> {
        let constraint = GrammarConstraint::JsonSchema {
            schema: r#"{"type": "string"}"#.to_owned(),
        };

        let json = serde_json::to_string(&constraint)?;
        let deserialized: GrammarConstraint = serde_json::from_str(&json)?;

        assert_eq!(constraint, deserialized);

        Ok(())
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let json =
            r#"{"type": "Gbnf", "grammar": "root ::= \"x\"", "root": "root", "extra": true}"#;
        let result = serde_json::from_str::<GrammarConstraint>(json);

        assert!(result.is_err());
    }
}
