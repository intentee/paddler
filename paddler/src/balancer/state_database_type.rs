use std::path::Path;
use std::path::PathBuf;
use std::path::absolute;
use std::str::FromStr;

use anyhow::Error;
use anyhow::Result;
use anyhow::anyhow;
use indoc::formatdoc;
use url::Url;

#[derive(Clone)]
pub enum StateDatabaseType {
    File(PathBuf),
    Memory,
}

impl FromStr for StateDatabaseType {
    type Err = Error;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(input)?;

        match url.scheme() {
            "file" => {
                let path = input
                    .strip_prefix("file://")
                    .ok_or_else(|| anyhow!("Invalid file URL: {input}"))?
                    .trim();

                if path.is_empty() {
                    return Err(anyhow!("File path cannot be empty"));
                }

                if !Path::new(path).is_absolute() {
                    let absolute_path = absolute(shellexpand::tilde(path).to_string())?;
                    let expanded_path = absolute_path.display();

                    return Err(anyhow!(formatdoc! {"
                        To avoid ambiguity, needing to guess the full file path (and to stay safe overall), Paddler requires absolute paths.
                        The path you wanted is *probably* '{expanded_path}'. If that is so, pass it as '--state-database file://{expanded_path}'.
                    "}));
                }

                Ok(StateDatabaseType::File(PathBuf::from(path)))
            }
            "memory" => Ok(StateDatabaseType::Memory),
            scheme => Err(anyhow!("Unsupported scheme '{scheme}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_memory_basic() {
        let result = StateDatabaseType::from_str("memory://").unwrap();
        assert!(matches!(result, StateDatabaseType::Memory));
    }

    #[test]
    fn test_file_relative_path() {
        let result = StateDatabaseType::from_str("file://path/to/db");

        assert!(result.is_err());
    }

    #[test]
    fn test_file_absolute_path() {
        let result = StateDatabaseType::from_str("file:///absolute/path").unwrap();
        match result {
            StateDatabaseType::File(path) => {
                assert_eq!(path, PathBuf::from("/absolute/path"));
            }
            _ => panic!("Expected File variant"),
        }
    }

    #[test]
    fn test_file_empty_path_fails() {
        let result = StateDatabaseType::from_str("file://");

        assert!(result.is_err());
    }

    #[test]
    fn test_unsupported_scheme() {
        let result = StateDatabaseType::from_str("mysql://localhost/db");

        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_url() {
        let result = StateDatabaseType::from_str("not-a-url");

        assert!(result.is_err());
    }
}
