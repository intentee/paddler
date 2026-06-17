use anyhow::Result;

use crate::required_env::required_env;

const IMAGE_NAME_ENV: &str = "PADDLER_TESTCONTAINER_IMAGE_NAME";
const IMAGE_TAG_ENV: &str = "PADDLER_TESTCONTAINER_IMAGE_TAG";

pub struct ImageReference {
    pub name: String,
    pub tag: String,
}

impl ImageReference {
    pub fn resolve() -> Result<Self> {
        Self::from_keys(IMAGE_NAME_ENV, IMAGE_TAG_ENV)
    }

    fn from_keys(name_key: &str, tag_key: &str) -> Result<Self> {
        Self::with_name(required_env(name_key)?, tag_key)
    }

    fn with_name(name: String, tag_key: &str) -> Result<Self> {
        Ok(Self {
            name,
            tag: required_env(tag_key)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ImageReference;

    const ABSENT_KEY: &str = "PADDLER_TESTCONTAINER_DELIBERATELY_ABSENT_VARIABLE";

    #[test]
    fn fails_when_image_name_is_absent() {
        assert!(ImageReference::from_keys(ABSENT_KEY, ABSENT_KEY).is_err());
    }

    #[test]
    fn fails_when_image_tag_is_absent() {
        assert!(ImageReference::with_name("paddler".to_owned(), ABSENT_KEY).is_err());
    }
}
