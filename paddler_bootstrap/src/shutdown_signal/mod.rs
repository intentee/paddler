#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::ShutdownSignals;
#[cfg(unix)]
pub use unix::register_shutdown_signals;
#[cfg(windows)]
pub use windows::ShutdownSignals;
#[cfg(windows)]
pub use windows::register_shutdown_signals;
