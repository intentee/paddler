use std::env;
use std::env::VarError;

use anyhow::Context as _;
use anyhow::Result;

const HF_HOME_ENV: &str = "HF_HOME";
const HOME_ENV: &str = "HOME";

pub fn host_huggingface_cache() -> Result<String> {
    match env::var(HF_HOME_ENV) {
        Ok(hf_home) => Ok(hf_home),
        Err(VarError::NotPresent) => {
            let home = env::var(HOME_ENV)
                .context("HOME is not set, cannot locate the Hugging Face cache directory")?;

            Ok(format!("{home}/.cache/huggingface"))
        }
        Err(error @ VarError::NotUnicode(_)) => {
            Err(anyhow::Error::new(error).context("HF_HOME is not valid unicode"))
        }
    }
}
