use std::io::stdout;

use tokio_util::sync::CancellationToken;
use trzcina::service_shutdown_options::ServiceShutdownOptions;

use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_bootstrap::registers_services::RegistersServices;
use paddler_bootstrap::run_service_manager::run_service_manager;

use crate::cli_error::CliError;

pub async fn run_announced_service_manager<TServices: RegistersServices>(
    addresses: BalancerAddresses,
    services: TServices,
    shutdown: CancellationToken,
) -> Result<(), CliError> {
    addresses
        .write_json_line(stdout().lock())
        .map_err(CliError::AddressesUnannounceable)?;

    run_service_manager(services, shutdown, ServiceShutdownOptions::default())
        .await
        .map_err(CliError::from)
}
