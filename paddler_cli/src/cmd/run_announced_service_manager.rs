use std::io::stdout;

use anyhow::Result;
use tokio_util::sync::CancellationToken;
use trzcina::ServiceShutdownOptions;

use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_bootstrap::registers_services::RegistersServices;
use paddler_bootstrap::run_service_manager::run_service_manager;

pub async fn run_announced_service_manager<TServices: RegistersServices>(
    addresses: BalancerAddresses,
    services: TServices,
    shutdown: CancellationToken,
) -> Result<()> {
    addresses.write_json_line(stdout().lock())?;

    run_service_manager(services, shutdown, ServiceShutdownOptions::default()).await
}
