use log::debug;
use tokio_util::sync::CancellationToken;

/// # Errors
///
/// Returns the `TError` of the remaining startup when it runs and fails.
pub fn continue_startup_unless_shutting_down<TError>(
    agent_shutdown: &CancellationToken,
    remaining_startup: impl FnOnce() -> Result<(), TError>,
) -> Result<(), TError> {
    if agent_shutdown.is_cancelled() {
        debug!(
            "Shutdown was requested during the scheduler startup; skipping its remaining stages"
        );

        return Ok(());
    }

    remaining_startup()
}
