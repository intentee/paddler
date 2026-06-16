use std::env;
use std::env::VarError;

use anyhow::Result;

const DEFAULT_IMAGE_NAME: &str = "ghcr.io/intentee/paddler";
const IMAGE_NAME_ENV: &str = "PADDLER_TESTCONTAINER_IMAGE_NAME";
const IMAGE_TAG_ENV: &str = "PADDLER_TESTCONTAINER_IMAGE_TAG";

fn env_or(key: &str, default: &str) -> Result<String> {
    match env::var(key) {
        Ok(value) => Ok(value),
        Err(VarError::NotPresent) => Ok(default.to_owned()),
        Err(error @ VarError::NotUnicode(_)) => Err(anyhow::Error::new(error)
            .context(format!("environment variable {key} is not valid unicode"))),
    }
}

pub struct ImageReference {
    pub name: String,
    pub tag: String,
}

impl ImageReference {
    pub fn resolve() -> Result<Self> {
        Ok(Self {
            name: env_or(IMAGE_NAME_ENV, DEFAULT_IMAGE_NAME)?,
            tag: env_or(IMAGE_TAG_ENV, env!("CARGO_PKG_VERSION"))?,
        })
    }
}
