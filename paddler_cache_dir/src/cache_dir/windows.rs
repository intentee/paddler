use std::env::var;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;

pub struct CacheDir {
    pub explicit: Option<String>,
    pub localappdata: Option<String>,
    pub userprofile: Option<String>,
}

impl CacheDir {
    #[must_use]
    pub fn from_process_env() -> Self {
        Self {
            explicit: var("PADDLER_CACHE_DIR").ok(),
            localappdata: var("LOCALAPPDATA").ok(),
            userprofile: var("USERPROFILE").ok(),
        }
    }

    pub fn resolve(&self) -> Result<PathBuf> {
        if let Some(explicit) = &self.explicit {
            return Ok(PathBuf::from(explicit));
        }

        if let Some(localappdata) = &self.localappdata {
            return Ok(PathBuf::from(localappdata).join("paddler"));
        }

        let userprofile = self
            .userprofile
            .as_ref()
            .context("USERPROFILE not set; cannot derive paddler cache directory")?;

        Ok(PathBuf::from(userprofile)
            .join("AppData")
            .join("Local")
            .join("paddler"))
    }
}

#[cfg(test)]
mod tests {
    use super::CacheDir;

    #[test]
    fn explicit_value_wins_over_localappdata_and_userprofile() {
        let cache = CacheDir {
            explicit: Some(r"D:\explicit\cache".to_owned()),
            localappdata: Some(r"C:\Users\user\AppData\Local".to_owned()),
            userprofile: Some(r"C:\Users\user".to_owned()),
        };
        let path = cache.resolve().unwrap_or_default();

        assert_eq!(path.to_string_lossy(), r"D:\explicit\cache");
    }

    #[test]
    fn localappdata_used_when_no_explicit() {
        let cache = CacheDir {
            explicit: None,
            localappdata: Some(r"C:\Users\user\AppData\Local".to_owned()),
            userprofile: Some(r"C:\Users\user".to_owned()),
        };
        let path = cache.resolve().unwrap_or_default();

        assert_eq!(
            path.to_string_lossy(),
            r"C:\Users\user\AppData\Local\paddler"
        );
    }

    #[test]
    fn falls_back_to_userprofile_appdata_local_paddler() {
        let cache = CacheDir {
            explicit: None,
            localappdata: None,
            userprofile: Some(r"C:\Users\user".to_owned()),
        };
        let path = cache.resolve().unwrap_or_default();

        assert_eq!(
            path.to_string_lossy(),
            r"C:\Users\user\AppData\Local\paddler"
        );
    }

    #[test]
    fn errors_when_no_env_set() {
        let cache = CacheDir {
            explicit: None,
            localappdata: None,
            userprofile: None,
        };

        assert!(cache.resolve().is_err());
    }

    #[test]
    fn from_process_env_constructs_without_panicking() {
        let _ = CacheDir::from_process_env();
    }
}
