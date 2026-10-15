use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::time::Duration;

use crate::inference_service::configuration::Configuration as InferenceServiceConfiguration;
use crate::resolved_socket_addr::ResolvedSocketAddr;

const INFERENCE_ITEM_TIMEOUT_LONGER_THAN_ANY_TEST: Duration = Duration::from_hours(1);

pub fn unexpiring_inference_service_configuration() -> InferenceServiceConfiguration {
    InferenceServiceConfiguration {
        addr: ResolvedSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))),
        cors_allowed_hosts: Vec::new(),
        inference_item_timeout: INFERENCE_ITEM_TIMEOUT_LONGER_THAN_ANY_TEST,
    }
}
