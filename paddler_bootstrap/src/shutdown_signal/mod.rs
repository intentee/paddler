#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::wait_for_shutdown_signal;
#[cfg(windows)]
pub use windows::wait_for_shutdown_signal;
