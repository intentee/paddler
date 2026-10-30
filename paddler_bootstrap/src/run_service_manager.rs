use anyhow::Error;
use anyhow::Result;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceManager;
use trzcina::ServiceShutdownOptions;

use crate::registers_services::RegistersServices;

pub async fn run_service_manager<TServices: RegistersServices>(
    services: TServices,
    task_shutdown: CancellationToken,
    shutdown_options: ServiceShutdownOptions,
) -> Result<()> {
    let mut service_manager = ServiceManager::default();

    services.register_services(&mut service_manager);
    service_manager
        .start(task_shutdown)
        .run_to_completion(shutdown_options)
        .await
        .into_result()
        .map_err(Error::from)
}
