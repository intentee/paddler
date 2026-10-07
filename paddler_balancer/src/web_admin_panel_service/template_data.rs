use std::time::Duration;

use paddler_messaging::inference_mode::InferenceMode;

use crate::resolved_socket_addr::ResolvedSocketAddr;
use crate::statsd_service::configuration::Configuration as StatsdServiceConfiguration;

#[derive(Clone)]
pub struct TemplateData {
    pub buffered_request_timeout: Duration,
    pub compat_openai_addr: Option<ResolvedSocketAddr>,
    pub compat_typesafe_addr: Option<ResolvedSocketAddr>,
    pub inference_addr: ResolvedSocketAddr,
    pub inference_mode: InferenceMode,
    pub management_addr: ResolvedSocketAddr,
    pub max_buffered_requests: u64,
    pub statsd_prefix: String,
    pub statsd_service_configuration: Option<StatsdServiceConfiguration>,
}

impl TemplateData {
    #[must_use]
    pub const fn inference_mode_name(&self) -> &'static str {
        match self.inference_mode {
            InferenceMode::Decision => "Decision",
            InferenceMode::Embeddings => "Embeddings",
            InferenceMode::TextGeneration => "TextGeneration",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;
    use std::time::Duration;

    use paddler_messaging::inference_mode::InferenceMode;

    use super::TemplateData;
    use crate::resolved_socket_addr::ResolvedSocketAddr;

    #[test]
    fn names_the_inference_mode_the_balancer_serves() {
        let loopback = ResolvedSocketAddr::from(SocketAddr::from((Ipv4Addr::LOCALHOST, 0)));

        assert_eq!(
            [
                InferenceMode::Decision,
                InferenceMode::Embeddings,
                InferenceMode::TextGeneration,
            ]
            .map(|inference_mode| TemplateData {
                buffered_request_timeout: Duration::from_secs(1),
                compat_openai_addr: None,
                compat_typesafe_addr: None,
                inference_addr: loopback.clone(),
                inference_mode,
                management_addr: loopback.clone(),
                max_buffered_requests: 1,
                statsd_prefix: "paddler_".to_owned(),
                statsd_service_configuration: None,
            }
            .inference_mode_name()),
            ["Decision", "Embeddings", "TextGeneration"]
        );
    }
}
