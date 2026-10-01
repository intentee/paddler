use serde_json::Map;
use serde_json::Number;
use serde_json::Value;
use yaml_rust2::yaml::Hash;
use yaml_rust2::yaml::Yaml;

use crate::openai_validator_error::OpenAIValidatorError;

fn real_to_value(real: &str) -> Result<Value, OpenAIValidatorError> {
    let parsed: f64 = real
        .parse()
        .map_err(|source| OpenAIValidatorError::YamlRealUnparsable {
            real: real.to_owned(),
            source,
        })?;

    let number =
        Number::from_f64(parsed).ok_or_else(|| OpenAIValidatorError::YamlRealNotFinite {
            real: real.to_owned(),
        })?;

    Ok(Value::Number(number))
}

fn hash_to_value(hash: &Hash) -> Result<Value, OpenAIValidatorError> {
    let mut object = Map::new();

    for (key, value) in hash {
        let Yaml::String(key) = key else {
            return Err(OpenAIValidatorError::YamlMappingKeyNotString { key: key.clone() });
        };

        object.insert(key.clone(), yaml_to_json_value(value)?);
    }

    Ok(Value::Object(object))
}

pub fn yaml_to_json_value(yaml: &Yaml) -> Result<Value, OpenAIValidatorError> {
    match yaml {
        Yaml::Null => Ok(Value::Null),
        Yaml::Boolean(boolean) => Ok(Value::Bool(*boolean)),
        Yaml::Integer(integer) => Ok(Value::Number(Number::from(*integer))),
        Yaml::Real(real) => real_to_value(real),
        Yaml::String(string) => Ok(Value::String(string.clone())),
        Yaml::Array(array) => array
            .iter()
            .map(yaml_to_json_value)
            .collect::<Result<Vec<Value>, OpenAIValidatorError>>()
            .map(Value::Array),
        Yaml::Hash(hash) => hash_to_value(hash),
        Yaml::Alias(index) => Err(OpenAIValidatorError::YamlAliasUnsupported { index: *index }),
        Yaml::BadValue => Err(OpenAIValidatorError::YamlBadValue),
    }
}

#[cfg(test)]
mod tests {
    use std::mem::discriminant;

    use serde_json::json;
    use yaml_rust2::yaml::Hash;
    use yaml_rust2::yaml::Yaml;

    use super::yaml_to_json_value;
    use crate::openai_validator_error::OpenAIValidatorError;

    #[test]
    fn converts_null() {
        assert_eq!(yaml_to_json_value(&Yaml::Null).unwrap(), json!(null));
    }

    #[test]
    fn converts_boolean() {
        assert_eq!(
            yaml_to_json_value(&Yaml::Boolean(true)).unwrap(),
            json!(true)
        );
    }

    #[test]
    fn converts_integer() {
        assert_eq!(yaml_to_json_value(&Yaml::Integer(42)).unwrap(), json!(42));
    }

    #[test]
    fn converts_string() {
        assert_eq!(
            yaml_to_json_value(&Yaml::String("hello".to_owned())).unwrap(),
            json!("hello")
        );
    }

    #[test]
    fn converts_real() {
        assert_eq!(
            yaml_to_json_value(&Yaml::Real("1.5".to_owned())).unwrap(),
            json!(1.5)
        );
    }

    #[test]
    fn rejects_unparseable_real() {
        let error = yaml_to_json_value(&Yaml::Real("not-a-number".to_owned())).unwrap_err();

        assert!(matches!(
            error,
            OpenAIValidatorError::YamlRealUnparsable { ref real, .. } if real == "not-a-number"
        ));
    }

    #[test]
    fn rejects_non_finite_real() {
        let error = yaml_to_json_value(&Yaml::Real("inf".to_owned())).unwrap_err();

        assert!(matches!(
            error,
            OpenAIValidatorError::YamlRealNotFinite { ref real } if real == "inf"
        ));
    }

    #[test]
    fn converts_array() {
        let array = Yaml::Array(vec![Yaml::Integer(1), Yaml::String("two".to_owned())]);

        assert_eq!(yaml_to_json_value(&array).unwrap(), json!([1, "two"]));
    }

    #[test]
    fn converts_hash_with_string_keys() {
        let mut hash = Hash::new();
        hash.insert(
            Yaml::String("name".to_owned()),
            Yaml::String("paddler".to_owned()),
        );

        assert_eq!(
            yaml_to_json_value(&Yaml::Hash(hash)).unwrap(),
            json!({"name": "paddler"})
        );
    }

    #[test]
    fn rejects_non_string_hash_keys() {
        let mut hash = Hash::new();
        hash.insert(Yaml::Integer(1), Yaml::Null);

        let error = yaml_to_json_value(&Yaml::Hash(hash)).unwrap_err();

        assert!(matches!(
            error,
            OpenAIValidatorError::YamlMappingKeyNotString { ref key } if *key == Yaml::Integer(1)
        ));
    }

    #[test]
    fn propagates_errors_from_hash_values() {
        let mut hash = Hash::new();
        hash.insert(Yaml::String("broken".to_owned()), Yaml::BadValue);

        let error = yaml_to_json_value(&Yaml::Hash(hash)).unwrap_err();

        assert_eq!(
            discriminant(&error),
            discriminant(&OpenAIValidatorError::YamlBadValue)
        );
    }

    #[test]
    fn propagates_errors_from_array_elements() {
        let array = Yaml::Array(vec![Yaml::BadValue]);

        let error = yaml_to_json_value(&array).unwrap_err();

        assert_eq!(
            discriminant(&error),
            discriminant(&OpenAIValidatorError::YamlBadValue)
        );
    }

    #[test]
    fn rejects_alias() {
        let error = yaml_to_json_value(&Yaml::Alias(7)).unwrap_err();

        assert!(matches!(
            error,
            OpenAIValidatorError::YamlAliasUnsupported { index } if index == 7
        ));
    }

    #[test]
    fn rejects_bad_value() {
        let error = yaml_to_json_value(&Yaml::BadValue).unwrap_err();

        assert_eq!(
            discriminant(&error),
            discriminant(&OpenAIValidatorError::YamlBadValue)
        );
    }
}
