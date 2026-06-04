use anyhow::Result;

use crate::balancer_addresses::BalancerAddresses;
use crate::managed_process::ManagedProcess;

pub struct RunningBalancer {
    pub addresses: BalancerAddresses,
    process: Box<dyn ManagedProcess>,
}

impl RunningBalancer {
    #[must_use]
    pub const fn new(addresses: BalancerAddresses, process: Box<dyn ManagedProcess>) -> Self {
        Self { addresses, process }
    }

    pub async fn shutdown(mut self) -> Result<()> {
        self.process.shutdown().await
    }
}
