use anyhow::Result;
use anyhow::anyhow;
use anyhow::bail;
use serde_json::Map;
use serde_json::Number;
use serde_json::Value;
use yaml_rust2::yaml::Hash;
use yaml_rust2::yaml::Yaml;

fn real_to_value(real: &str) -> Result<Value> {
    let parsed: f64 = real
        .parse()
        .map_err(|error| anyhow!("could not parse YAML real {real:?}: {error}"))?;

    let number =
        Number::from_f64(parsed).ok_or_else(|| anyhow!("YAML real {real:?} is not finite"))?;

    Ok(Value::Number(number))
}

fn hash_to_value(hash: &Hash) -> Result<Value> {
    let mut object = Map::new();

    for (key, value) in hash {
        let Yaml::String(key) = key else {
            bail!("YAML mapping keys must be strings, found {key:?}");
        };

        object.insert(key.clone(), yaml_to_json_value(value)?);
    }

    Ok(Value::Object(object))
}

pub fn yaml_to_json_value(yaml: &Yaml) -> Result<Value> {
    match yaml {
        Yaml::Null => Ok(Value::Null),
        Yaml::Boolean(boolean) => Ok(Value::Bool(*boolean)),
        Yaml::Integer(integer) => Ok(Value::Number(Number::from(*integer))),
        Yaml::Real(real) => real_to_value(real),
        Yaml::String(string) => Ok(Value::String(string.clone())),
        Yaml::Array(array) => array
            .iter()
            .map(yaml_to_json_value)
            .collect::<Result<Vec<Value>>>()
            .map(Value::Array),
        Yaml::Hash(hash) => hash_to_value(hash),
        Yaml::Alias(index) => bail!("YAML aliases are not supported (alias #{index})"),
        Yaml::BadValue => bail!("encountered an invalid YAML node"),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use yaml_rust2::yaml::Hash;
    use yaml_rust2::yaml::Yaml;

    use super::yaml_to_json_value;

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

        assert!(error.to_string().contains("could not parse YAML real"));
    }

    #[test]
    fn rejects_non_finite_real() {
        let error = yaml_to_json_value(&Yaml::Real("inf".to_owned())).unwrap_err();

        assert!(error.to_string().contains("not finite"));
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

        assert!(error.to_string().contains("mapping keys must be strings"));
    }

    #[test]
    fn propagates_errors_from_hash_values() {
        let mut hash = Hash::new();
        hash.insert(Yaml::String("broken".to_owned()), Yaml::BadValue);

        let error = yaml_to_json_value(&Yaml::Hash(hash)).unwrap_err();

        assert!(error.to_string().contains("invalid YAML node"));
    }

    #[test]
    fn propagates_errors_from_array_elements() {
        let array = Yaml::Array(vec![Yaml::BadValue]);

        let error = yaml_to_json_value(&array).unwrap_err();

        assert!(error.to_string().contains("invalid YAML node"));
    }

    #[test]
    fn rejects_alias() {
        let error = yaml_to_json_value(&Yaml::Alias(7)).unwrap_err();

        assert!(error.to_string().contains("aliases are not supported"));
    }

    #[test]
    fn rejects_bad_value() {
        let error = yaml_to_json_value(&Yaml::BadValue).unwrap_err();

        assert!(error.to_string().contains("invalid YAML node"));
    }
}
