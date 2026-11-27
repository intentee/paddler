use tokio_util::sync::CancellationToken;
use trzcina::service_manager::ServiceManager;
use trzcina::service_shutdown_options::ServiceShutdownOptions;

use crate::bootstrap_error::BootstrapError;
use crate::registers_services::RegistersServices;

/// # Errors
///
/// Returns [`BootstrapError::ServiceRunFailed`] when one of the registered services stops with an error, panics, or does not stop before the shutdown deadline.
pub async fn run_service_manager<TServices: RegistersServices>(
    services: TServices,
    task_shutdown: CancellationToken,
    shutdown_options: ServiceShutdownOptions,
) -> Result<(), BootstrapError> {
    let mut service_manager = ServiceManager::default();

    services.register_services(&mut service_manager);
    service_manager
        .start(task_shutdown)
        .run_to_completion(shutdown_options)
        .await
        .into_result()
        .map_err(|failures| BootstrapError::ServiceRunFailed { failures })
}
