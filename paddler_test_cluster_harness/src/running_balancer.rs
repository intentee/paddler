use std::net::SocketAddr;

use anyhow::Result;
use url::Url;

use paddler_balancer::balancer_addresses::BalancerAddresses;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::managed_process::ManagedProcess;

fn base_url_for(addr: SocketAddr) -> Result<Url, ClusterHarnessError> {
    Url::parse(&format!("http://{addr}/"))
        .map_err(|source| ClusterHarnessError::BaseUrlInvalid { addr, source })
}

pub struct RunningBalancer {
    pub addresses: BalancerAddresses,
    process: Box<dyn ManagedProcess>,
}

impl RunningBalancer {
    #[must_use]
    pub const fn new(addresses: BalancerAddresses, process: Box<dyn ManagedProcess>) -> Self {
        Self { addresses, process }
    }

    pub fn compat_openai_addr(&self) -> Result<SocketAddr, ClusterHarnessError> {
        self.addresses
            .compat_openai
            .ok_or(ClusterHarnessError::CompatOpenAIServiceNotServed)
    }

    pub fn compat_openai_base_url(&self) -> Result<Url, ClusterHarnessError> {
        self.compat_openai_addr().and_then(base_url_for)
    }

    pub fn compat_typesafe_addr(&self) -> Result<SocketAddr, ClusterHarnessError> {
        self.addresses
            .compat_typesafe
            .ok_or(ClusterHarnessError::CompatTypeSafeServiceNotServed)
    }

    pub fn compat_typesafe_base_url(&self) -> Result<Url, ClusterHarnessError> {
        self.compat_typesafe_addr().and_then(base_url_for)
    }

    pub fn inference_base_url(&self) -> Result<Url, ClusterHarnessError> {
        base_url_for(self.addresses.inference)
    }

    pub fn management_base_url(&self) -> Result<Url, ClusterHarnessError> {
        base_url_for(self.addresses.management)
    }

    pub async fn shutdown(self) -> Result<()> {
        self.process.shutdown().await
    }
}
