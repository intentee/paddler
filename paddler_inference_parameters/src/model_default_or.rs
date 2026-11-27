use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(from = "Option<TValue>", into = "Option<TValue>")]
pub enum ModelDefaultOr<TValue: Clone> {
    Explicit(TValue),
    #[default]
    ModelDefault,
}

impl<TValue: Clone> ModelDefaultOr<TValue> {
    #[must_use]
    pub fn or(self, model_default: Self) -> Self {
        match self {
            Self::Explicit(value) => Self::Explicit(value),
            Self::ModelDefault => model_default,
        }
    }

    #[must_use]
    pub fn map<TMapped: Clone>(
        self,
        map_explicit: impl FnOnce(TValue) -> TMapped,
    ) -> ModelDefaultOr<TMapped> {
        match self {
            Self::Explicit(value) => ModelDefaultOr::Explicit(map_explicit(value)),
            Self::ModelDefault => ModelDefaultOr::ModelDefault,
        }
    }

    #[must_use]
    pub fn resolve(self, model_default: TValue) -> TValue {
        match self {
            Self::Explicit(value) => value,
            Self::ModelDefault => model_default,
        }
    }
}

impl<TValue: Clone> From<Option<TValue>> for ModelDefaultOr<TValue> {
    fn from(requested_value: Option<TValue>) -> Self {
        requested_value.map_or(Self::ModelDefault, Self::Explicit)
    }
}

impl<TValue: Clone> From<ModelDefaultOr<TValue>> for Option<TValue> {
    fn from(choice: ModelDefaultOr<TValue>) -> Self {
        match choice {
            ModelDefaultOr::Explicit(value) => Some(value),
            ModelDefaultOr::ModelDefault => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::from_value;
    use serde_json::json;
    use serde_json::to_value;

    use super::ModelDefaultOr;

    #[test]
    fn an_explicit_value_overrides_the_model_default() {
        assert_eq!(ModelDefaultOr::Explicit(0.2_f32).resolve(0.7), 0.2);
        assert_eq!(ModelDefaultOr::ModelDefault.resolve(0.7_f32), 0.7);
    }

    #[test]
    fn an_explicit_value_is_kept_over_what_the_model_settled_on() {
        assert_eq!(
            ModelDefaultOr::Explicit(4096_u32).or(ModelDefaultOr::Explicit(8192)),
            ModelDefaultOr::Explicit(4096)
        );
        assert_eq!(
            ModelDefaultOr::ModelDefault.or(ModelDefaultOr::Explicit(8192_u32)),
            ModelDefaultOr::Explicit(8192)
        );
    }

    #[test]
    fn mapping_converts_only_an_explicit_value() {
        assert_eq!(
            ModelDefaultOr::Explicit(2048_u32).map(u64::from),
            ModelDefaultOr::Explicit(2048_u64)
        );
        assert_eq!(
            ModelDefaultOr::<u32>::ModelDefault.map(u64::from),
            ModelDefaultOr::ModelDefault
        );
    }

    #[test]
    fn null_on_the_wire_chooses_the_model_default() {
        assert_eq!(
            from_value::<ModelDefaultOr<u32>>(json!(null)).unwrap(),
            ModelDefaultOr::ModelDefault
        );
        assert_eq!(
            from_value::<ModelDefaultOr<u32>>(json!(4096)).unwrap(),
            ModelDefaultOr::Explicit(4096)
        );
        assert_eq!(
            to_value(ModelDefaultOr::<u32>::ModelDefault).unwrap(),
            json!(null)
        );
    }
}
