use std::path::Path;
use std::path::PathBuf;
use std::path::absolute;
use std::str::FromStr;

use shellexpand::tilde;
use url::Url;

use paddler_messaging::balancer_desired_state::BalancerDesiredState;

use crate::state_database_error::StateDatabaseError;

fn relative_path_error(path: &str) -> StateDatabaseError {
    match absolute(tilde(path).to_string()) {
        Ok(suggested_path) => StateDatabaseError::FilePathRelative { suggested_path },
        Err(source) => StateDatabaseError::FilePathRelativeUnresolvable {
            path: path.to_owned(),
            source,
        },
    }
}

#[derive(Clone)]
pub enum StateDatabaseType {
    File(PathBuf),
    Memory(Box<BalancerDesiredState>),
}

impl FromStr for StateDatabaseType {
    type Err = StateDatabaseError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(input).map_err(|source| StateDatabaseError::UrlInvalid {
            input: input.to_owned(),
            source,
        })?;

        match url.scheme() {
            "file" => {
                let path = input
                    .strip_prefix("file://")
                    .ok_or_else(|| StateDatabaseError::FileUrlMalformed {
                        input: input.to_owned(),
                    })?
                    .trim();

                if path.is_empty() {
                    return Err(StateDatabaseError::FilePathEmpty);
                }

                if !Path::new(path).is_absolute() {
                    return Err(relative_path_error(path));
                }

                Ok(Self::File(PathBuf::from(path)))
            }
            "memory" => Ok(Self::Memory(Box::default())),
            scheme => Err(StateDatabaseError::SchemeUnsupported {
                scheme: scheme.to_owned(),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::env::set_current_dir;
    use std::mem::discriminant;
    use std::path::PathBuf;
    use std::str::FromStr;

    use tempfile::TempDir;

    use paddler_messaging::balancer_desired_state::BalancerDesiredState;

    use super::StateDatabaseType;
    use crate::state_database_error::StateDatabaseError;

    #[test]
    fn parses_a_memory_url_into_the_default_state() {
        let result = StateDatabaseType::from_str("memory://").unwrap();

        assert!(matches!(
            result,
            StateDatabaseType::Memory(initial_desired_state)
                if *initial_desired_state == BalancerDesiredState::default()
        ));
    }

    #[test]
    fn rejects_a_file_url_with_a_relative_path() {
        let result = StateDatabaseType::from_str("file://path/to/db");

        assert!(matches!(
            result,
            Err(StateDatabaseError::FilePathRelative { suggested_path })
                if suggested_path.is_absolute() && suggested_path.ends_with("path/to/db")
        ));
    }

    #[test]
    fn reports_a_relative_path_that_cannot_be_resolved_without_a_working_directory() {
        let removed_working_directory = TempDir::new().unwrap();

        set_current_dir(removed_working_directory.path()).unwrap();
        removed_working_directory.close().unwrap();

        assert!(matches!(
            StateDatabaseType::from_str("file://path/to/db"),
            Err(StateDatabaseError::FilePathRelativeUnresolvable { path, .. }) if path == "path/to/db"
        ));
    }

    #[test]
    fn rejects_a_file_url_without_an_authority() {
        let result = StateDatabaseType::from_str("file:relative");

        assert!(matches!(
            result,
            Err(StateDatabaseError::FileUrlMalformed { input }) if input == "file:relative"
        ));
    }

    #[test]
    fn parses_a_file_url_with_an_absolute_path() {
        let result = StateDatabaseType::from_str("file:///absolute/path").unwrap();

        assert!(
            matches!(&result, StateDatabaseType::File(path) if path == &PathBuf::from("/absolute/path"))
        );
    }

    #[test]
    fn rejects_a_file_url_without_a_path() {
        let result = StateDatabaseType::from_str("file://");

        assert_eq!(
            result.err().as_ref().map(discriminant),
            Some(discriminant(&StateDatabaseError::FilePathEmpty))
        );
    }

    #[test]
    fn rejects_an_unsupported_scheme() {
        let result = StateDatabaseType::from_str("mysql://localhost/db");

        assert!(matches!(
            result,
            Err(StateDatabaseError::SchemeUnsupported { scheme }) if scheme == "mysql"
        ));
    }

    #[test]
    fn rejects_an_unparsable_url() {
        let result = StateDatabaseType::from_str("not-a-url");

        assert!(matches!(
            result,
            Err(StateDatabaseError::UrlInvalid { input, .. }) if input == "not-a-url"
        ));
    }
}
