use std::env;
use std::env::VarError;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;

fn env_var_error(error: VarError, key: &str) -> Error {
    match error {
        VarError::NotPresent => anyhow!("environment variable {key} must be set"),
        VarError::NotUnicode(_) => anyhow!("environment variable {key} is not valid unicode"),
    }
}

pub fn required_env(key: &str) -> Result<String> {
    env::var(key).map_err(|error| env_var_error(error, key))
}

#[cfg(test)]
mod tests {
    use std::env::VarError;
    use std::ffi::OsString;

    use super::env_var_error;
    use super::required_env;

    const ABSENT_KEY: &str = "PADDLER_TESTCONTAINER_DELIBERATELY_ABSENT_VARIABLE";

    #[test]
    fn reports_absent_required_variable() {
        let error_message = required_env(ABSENT_KEY)
            .err()
            .map(|error| error.to_string());

        assert_eq!(
            error_message,
            Some(format!("environment variable {ABSENT_KEY} must be set")),
        );
    }

    #[test]
    fn reports_non_unicode_variable() {
        let error = env_var_error(VarError::NotUnicode(OsString::from("value")), "EXAMPLE_KEY");

        assert_eq!(
            error.to_string(),
            "environment variable EXAMPLE_KEY is not valid unicode",
        );
    }
}
