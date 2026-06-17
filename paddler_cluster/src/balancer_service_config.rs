use std::time::Duration;

pub struct BalancerServiceConfig {
    pub buffered_request_timeout: Duration,
    pub inference_cors_allowed_hosts: Vec<String>,
    pub inference_item_timeout: Duration,
    pub management_cors_allowed_hosts: Vec<String>,
    pub max_buffered_requests: i32,
    pub state_database_url: String,
}

impl BalancerServiceConfig {
    #[must_use]
    pub fn command_args(&self) -> Vec<String> {
        let Self {
            buffered_request_timeout,
            inference_cors_allowed_hosts,
            inference_item_timeout,
            management_cors_allowed_hosts,
            max_buffered_requests,
            state_database_url,
        } = self;

        let mut args = vec![
            "--state-database".to_owned(),
            state_database_url.clone(),
            "--max-buffered-requests".to_owned(),
            max_buffered_requests.to_string(),
            "--buffered-request-timeout".to_owned(),
            buffered_request_timeout.as_millis().to_string(),
            "--inference-item-timeout".to_owned(),
            inference_item_timeout.as_millis().to_string(),
        ];

        for allowed_host in inference_cors_allowed_hosts {
            args.push("--inference-cors-allowed-host".to_owned());
            args.push(allowed_host.clone());
        }

        for allowed_host in management_cors_allowed_hosts {
            args.push("--management-cors-allowed-host".to_owned());
            args.push(allowed_host.clone());
        }

        args
    }
}

impl Default for BalancerServiceConfig {
    fn default() -> Self {
        Self {
            buffered_request_timeout: Duration::from_secs(10),
            inference_cors_allowed_hosts: Vec::new(),
            inference_item_timeout: Duration::from_secs(30),
            management_cors_allowed_hosts: Vec::new(),
            max_buffered_requests: 10,
            state_database_url: "memory://".to_owned(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BalancerServiceConfig;

    #[test]
    fn renders_default_service_flags_without_cors_hosts() {
        let args = BalancerServiceConfig::default().command_args();

        assert_eq!(
            args,
            vec![
                "--state-database".to_owned(),
                "memory://".to_owned(),
                "--max-buffered-requests".to_owned(),
                "10".to_owned(),
                "--buffered-request-timeout".to_owned(),
                "10000".to_owned(),
                "--inference-item-timeout".to_owned(),
                "30000".to_owned(),
            ],
        );
    }

    #[test]
    fn renders_repeated_cors_host_flags() {
        let args = BalancerServiceConfig {
            inference_cors_allowed_hosts: vec!["https://inference.example".to_owned()],
            management_cors_allowed_hosts: vec![
                "https://manage-a.example".to_owned(),
                "https://manage-b.example".to_owned(),
            ],
            ..BalancerServiceConfig::default()
        }
        .command_args();

        assert_eq!(
            args.iter()
                .filter(|arg| arg.as_str() == "--inference-cors-allowed-host")
                .count(),
            1,
        );
        assert_eq!(
            args.iter()
                .filter(|arg| arg.as_str() == "--management-cors-allowed-host")
                .count(),
            2,
        );
        assert!(args.contains(&"https://inference.example".to_owned()));
        assert!(args.contains(&"https://manage-b.example".to_owned()));
    }
}
