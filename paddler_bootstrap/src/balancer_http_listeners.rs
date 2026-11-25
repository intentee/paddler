use std::io;
use std::net::SocketAddr;

use paddler_balancer::balancer_addresses::BalancerAddresses;
use paddler_balancer::compatibility::compatibility_service_configuration::CompatibilityServiceConfiguration;
use paddler_balancer::http_listener::HttpListener;
use paddler_balancer::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use paddler_balancer::management_service::configuration::Configuration as ManagementServiceConfiguration;

use crate::bootstrap_error::BootstrapError;

fn bind_http_listener(
    addr: SocketAddr,
    bind_failed: fn(SocketAddr, io::Error) -> BootstrapError,
) -> Result<HttpListener, BootstrapError> {
    HttpListener::bind(addr).map_err(|source| bind_failed(addr, source))
}

fn bind_compatibility_http_listener(
    compatibility_service_configuration: Option<&CompatibilityServiceConfiguration>,
    bind_failed: fn(SocketAddr, io::Error) -> BootstrapError,
) -> Result<Option<HttpListener>, BootstrapError> {
    compatibility_service_configuration
        .map(|CompatibilityServiceConfiguration { addr }| {
            bind_http_listener(addr.socket_addr, bind_failed)
        })
        .transpose()
}

pub struct BalancerHttpListeners {
    pub compat_openai: Option<HttpListener>,
    pub compat_typesafe: Option<HttpListener>,
    pub inference: HttpListener,
    pub management: HttpListener,
}

impl BalancerHttpListeners {
    /// # Errors
    ///
    /// Returns [`BootstrapError::InferenceBindFailed`], [`BootstrapError::ManagementBindFailed`], [`BootstrapError::CompatOpenAIBindFailed`] or [`BootstrapError::CompatTypeSafeBindFailed`] when the address of the corresponding service cannot be bound, binding them in that order.
    pub fn bind(
        InferenceServiceConfiguration {
            addr: inference_addr,
            ..
        }: &InferenceServiceConfiguration,
        ManagementServiceConfiguration {
            addr: management_addr,
            ..
        }: &ManagementServiceConfiguration,
        openai_service_configuration: Option<&CompatibilityServiceConfiguration>,
        typesafe_service_configuration: Option<&CompatibilityServiceConfiguration>,
    ) -> Result<Self, BootstrapError> {
        let inference = bind_http_listener(inference_addr.socket_addr, |addr, source| {
            BootstrapError::InferenceBindFailed { addr, source }
        })?;
        let management = bind_http_listener(management_addr.socket_addr, |addr, source| {
            BootstrapError::ManagementBindFailed { addr, source }
        })?;
        let compat_openai =
            bind_compatibility_http_listener(openai_service_configuration, |addr, source| {
                BootstrapError::CompatOpenAIBindFailed { addr, source }
            })?;
        let compat_typesafe =
            bind_compatibility_http_listener(typesafe_service_configuration, |addr, source| {
                BootstrapError::CompatTypeSafeBindFailed { addr, source }
            })?;

        Ok(Self {
            compat_openai,
            compat_typesafe,
            inference,
            management,
        })
    }

    #[must_use]
    pub fn addresses(&self) -> BalancerAddresses {
        BalancerAddresses {
            compat_openai: self
                .compat_openai
                .as_ref()
                .map(|&HttpListener { local_addr, .. }| local_addr),
            compat_typesafe: self
                .compat_typesafe
                .as_ref()
                .map(|&HttpListener { local_addr, .. }| local_addr),
            inference: self.inference.local_addr,
            management: self.management.local_addr,
            web_admin_panel: None,
        }
    }
}
