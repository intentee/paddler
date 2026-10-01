use std::env::var_os;
use std::path::PathBuf;

use crate::cache_dir_error::CacheDirError;

pub struct CacheDir {
    pub explicit: Option<PathBuf>,
    pub home: Option<PathBuf>,
    pub xdg: Option<PathBuf>,
}

impl CacheDir {
    #[must_use]
    pub fn from_process_env() -> Self {
        Self {
            explicit: var_os("PADDLER_CACHE_DIR").map(PathBuf::from),
            home: var_os("HOME").map(PathBuf::from),
            xdg: var_os("XDG_CACHE_HOME").map(PathBuf::from),
        }
    }

    pub fn resolve(&self) -> Result<PathBuf, CacheDirError> {
        if let Some(explicit) = &self.explicit {
            return Ok(explicit.clone());
        }

        if let Some(xdg) = &self.xdg {
            return Ok(xdg.join("paddler"));
        }

        self.home
            .as_ref()
            .map(|home| home.join(".cache").join("paddler"))
            .ok_or(CacheDirError::HomeVariableUnset { variable: "HOME" })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CacheDir;
    use crate::cache_dir_error::CacheDirError;

    #[test]
    fn explicit_value_wins_over_xdg_and_home() {
        let cache = CacheDir {
            explicit: Some(PathBuf::from("/explicit/cache")),
            home: Some(PathBuf::from("/home/user")),
            xdg: Some(PathBuf::from("/xdg/cache")),
        };
        let path = cache.resolve().unwrap();

        assert_eq!(path, PathBuf::from("/explicit/cache"));
    }

    #[test]
    fn xdg_value_used_when_no_explicit() {
        let cache = CacheDir {
            explicit: None,
            home: Some(PathBuf::from("/home/user")),
            xdg: Some(PathBuf::from("/xdg/cache")),
        };
        let path = cache.resolve().unwrap();

        assert_eq!(path, PathBuf::from("/xdg/cache/paddler"));
    }

    #[test]
    fn falls_back_to_home_dot_cache_paddler() {
        let cache = CacheDir {
            explicit: None,
            home: Some(PathBuf::from("/home/user")),
            xdg: None,
        };
        let path = cache.resolve().unwrap();

        assert_eq!(path, PathBuf::from("/home/user/.cache/paddler"));
    }

    #[test]
    fn errors_when_no_env_set() {
        let cache = CacheDir {
            explicit: None,
            home: None,
            xdg: None,
        };

        assert!(matches!(
            cache.resolve(),
            Err(CacheDirError::HomeVariableUnset { variable }) if variable == "HOME"
        ));
    }
}
