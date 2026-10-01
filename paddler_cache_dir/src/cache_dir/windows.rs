use std::env::var_os;
use std::path::PathBuf;

use crate::cache_dir_error::CacheDirError;

pub struct CacheDir {
    pub explicit: Option<PathBuf>,
    pub localappdata: Option<PathBuf>,
    pub userprofile: Option<PathBuf>,
}

impl CacheDir {
    #[must_use]
    pub fn from_process_env() -> Self {
        Self {
            explicit: var_os("PADDLER_CACHE_DIR").map(PathBuf::from),
            localappdata: var_os("LOCALAPPDATA").map(PathBuf::from),
            userprofile: var_os("USERPROFILE").map(PathBuf::from),
        }
    }

    pub fn resolve(&self) -> Result<PathBuf, CacheDirError> {
        if let Some(explicit) = &self.explicit {
            return Ok(explicit.clone());
        }

        if let Some(localappdata) = &self.localappdata {
            return Ok(localappdata.join("paddler"));
        }

        self.userprofile
            .as_ref()
            .map(|userprofile| userprofile.join("AppData").join("Local").join("paddler"))
            .ok_or(CacheDirError::HomeVariableUnset {
                variable: "USERPROFILE",
            })
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CacheDir;
    use crate::cache_dir_error::CacheDirError;

    #[test]
    fn explicit_value_wins_over_localappdata_and_userprofile() {
        let cache = CacheDir {
            explicit: Some(PathBuf::from(r"D:\explicit\cache")),
            localappdata: Some(PathBuf::from(r"C:\Users\user\AppData\Local")),
            userprofile: Some(PathBuf::from(r"C:\Users\user")),
        };
        let path = cache.resolve().unwrap();

        assert_eq!(path, PathBuf::from(r"D:\explicit\cache"));
    }

    #[test]
    fn localappdata_used_when_no_explicit() {
        let cache = CacheDir {
            explicit: None,
            localappdata: Some(PathBuf::from(r"C:\Users\user\AppData\Local")),
            userprofile: Some(PathBuf::from(r"C:\Users\user")),
        };
        let path = cache.resolve().unwrap();

        assert_eq!(path, PathBuf::from(r"C:\Users\user\AppData\Local\paddler"));
    }

    #[test]
    fn falls_back_to_userprofile_appdata_local_paddler() {
        let cache = CacheDir {
            explicit: None,
            localappdata: None,
            userprofile: Some(PathBuf::from(r"C:\Users\user")),
        };
        let path = cache.resolve().unwrap();

        assert_eq!(path, PathBuf::from(r"C:\Users\user\AppData\Local\paddler"));
    }

    #[test]
    fn errors_when_no_env_set() {
        let cache = CacheDir {
            explicit: None,
            localappdata: None,
            userprofile: None,
        };

        assert!(matches!(
            cache.resolve(),
            Err(CacheDirError::HomeVariableUnset { variable }) if variable == "USERPROFILE"
        ));
    }
}
