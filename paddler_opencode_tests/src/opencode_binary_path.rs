use std::env;
use std::env::VarError;
use std::path::PathBuf;

use crate::opencode_test_error::OpenCodeTestError;

const OPENCODE_BINARY_ENV: &str = "PADDLER_OPENCODE_BINARY";

fn resolve_binary_path(raw: Result<String, VarError>) -> Result<PathBuf, OpenCodeTestError> {
    match raw {
        Ok(raw_path) => {
            let path = PathBuf::from(raw_path);

            if path.exists() {
                Ok(path)
            } else {
                Err(OpenCodeTestError::BinaryDoesNotExist { path })
            }
        }
        Err(source) => Err(OpenCodeTestError::BinaryPathNotProvided {
            variable: OPENCODE_BINARY_ENV.to_owned(),
            source,
        }),
    }
}

pub fn opencode_binary_path() -> Result<PathBuf, OpenCodeTestError> {
    resolve_binary_path(env::var(OPENCODE_BINARY_ENV))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_environment_variable_reports_not_provided() {
        let error = resolve_binary_path(Err(VarError::NotPresent)).unwrap_err();

        assert!(matches!(
            error,
            OpenCodeTestError::BinaryPathNotProvided { variable, .. } if variable == OPENCODE_BINARY_ENV
        ));
    }

    #[test]
    fn nonexistent_path_reports_missing_binary() {
        let error =
            resolve_binary_path(Ok("/paddler/definitely/missing/opencode".to_owned())).unwrap_err();

        assert!(matches!(
            error,
            OpenCodeTestError::BinaryDoesNotExist { .. }
        ));
    }

    #[test]
    fn existing_path_is_returned() {
        let binary = tempfile::NamedTempFile::new().unwrap();

        let resolved =
            resolve_binary_path(Ok(binary.path().to_string_lossy().into_owned())).unwrap();

        assert_eq!(resolved, binary.path());
    }
}
