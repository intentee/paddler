use std::net::SocketAddr;

use anyhow::Result;
use url::ParseError;
use url::Url;

use paddler_balancer::balancer_addresses::BalancerAddresses;

use crate::cluster_harness_error::ClusterHarnessError;
use crate::managed_process::ManagedProcess;

fn base_url_for(addr: SocketAddr) -> Result<Url, ParseError> {
    Url::parse(&format!("http://{addr}/"))
}

pub struct RunningBalancer {
    pub addresses: BalancerAddresses,
    pub inference_base_url: Url,
    pub management_base_url: Url,
    process: Box<dyn ManagedProcess>,
}

impl RunningBalancer {
    pub fn new(
        addresses: BalancerAddresses,
        process: Box<dyn ManagedProcess>,
    ) -> Result<Self, ClusterHarnessError> {
        base_url_for(addresses.inference)
            .map_err(ClusterHarnessError::InferenceBaseUrlInvalid)
            .and_then(|inference_base_url| {
                base_url_for(addresses.management)
                    .map_err(ClusterHarnessError::ManagementBaseUrlInvalid)
                    .map(|management_base_url| Self {
                        addresses,
                        inference_base_url,
                        management_base_url,
                        process,
                    })
            })
    }

    pub fn compat_openai_addr(&self) -> Result<SocketAddr, ClusterHarnessError> {
        self.addresses
            .compat_openai
            .ok_or(ClusterHarnessError::CompatOpenAIServiceNotServed)
    }

    pub fn compat_openai_base_url(&self) -> Result<Url, ClusterHarnessError> {
        self.compat_openai_addr().and_then(|addr| {
            base_url_for(addr).map_err(ClusterHarnessError::CompatOpenAIBaseUrlInvalid)
        })
    }

    pub fn compat_openai_url(&self, path: &str) -> Result<Url, ClusterHarnessError> {
        let mut compat_openai_url = self.compat_openai_base_url()?;

        compat_openai_url.set_path(path);

        Ok(compat_openai_url)
    }

    pub fn compat_typesafe_addr(&self) -> Result<SocketAddr, ClusterHarnessError> {
        self.addresses
            .compat_typesafe
            .ok_or(ClusterHarnessError::CompatTypeSafeServiceNotServed)
    }

    pub fn compat_typesafe_base_url(&self) -> Result<Url, ClusterHarnessError> {
        self.compat_typesafe_addr().and_then(|addr| {
            base_url_for(addr).map_err(ClusterHarnessError::CompatTypeSafeBaseUrlInvalid)
        })
    }

    pub fn compat_typesafe_url(&self, path: &str) -> Result<Url, ClusterHarnessError> {
        let mut compat_typesafe_url = self.compat_typesafe_base_url()?;

        compat_typesafe_url.set_path(path);

        Ok(compat_typesafe_url)
    }

    pub async fn shutdown(self) -> Result<()> {
        self.process.shutdown().await
    }
}
