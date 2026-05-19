#[cfg(unix)]
mod unix;
#[cfg(unix)]
pub use crate::cache_dir::unix::CacheDir;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use crate::cache_dir::windows::CacheDir;
