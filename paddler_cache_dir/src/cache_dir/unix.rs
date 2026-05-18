use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;

pub struct CacheDir {
    pub explicit: Option<String>,
    pub home: Option<String>,
    pub xdg: Option<String>,
}

impl CacheDir {
    #[must_use]
    pub fn from_process_env() -> Self {
        Self {
            explicit: std::env::var("PADDLER_CACHE_DIR").ok(),
            home: std::env::var("HOME").ok(),
            xdg: std::env::var("XDG_CACHE_HOME").ok(),
        }
    }

    pub fn resolve(&self) -> Result<PathBuf> {
        if let Some(explicit) = &self.explicit {
            return Ok(PathBuf::from(explicit));
        }

        if let Some(xdg) = &self.xdg {
            return Ok(PathBuf::from(xdg).join("paddler"));
        }

        let home = self
            .home
            .as_ref()
            .context("HOME not set; cannot derive paddler cache directory")?;

        Ok(PathBuf::from(home).join(".cache").join("paddler"))
    }
}

#[cfg(test)]
mod tests {
    use crate::cache_dir::unix::CacheDir;

    #[test]
    fn explicit_value_wins_over_xdg_and_home() {
        let cache = CacheDir {
            explicit: Some("/explicit/cache".to_owned()),
            home: Some("/home/user".to_owned()),
            xdg: Some("/xdg/cache".to_owned()),
        };
        let path = cache.resolve().unwrap_or_default();

        assert_eq!(path.to_string_lossy(), "/explicit/cache");
    }

    #[test]
    fn xdg_value_used_when_no_explicit() {
        let cache = CacheDir {
            explicit: None,
            home: Some("/home/user".to_owned()),
            xdg: Some("/xdg/cache".to_owned()),
        };
        let path = cache.resolve().unwrap_or_default();

        assert_eq!(path.to_string_lossy(), "/xdg/cache/paddler");
    }

    #[test]
    fn falls_back_to_home_dot_cache_paddler() {
        let cache = CacheDir {
            explicit: None,
            home: Some("/home/user".to_owned()),
            xdg: None,
        };
        let path = cache.resolve().unwrap_or_default();

        assert_eq!(path.to_string_lossy(), "/home/user/.cache/paddler");
    }

    #[test]
    fn errors_when_no_env_set() {
        let cache = CacheDir {
            explicit: None,
            home: None,
            xdg: None,
        };

        assert!(cache.resolve().is_err());
    }

    #[test]
    fn from_process_env_constructs_without_panicking() {
        let _ = CacheDir::from_process_env();
    }
}
