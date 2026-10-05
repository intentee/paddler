use std::net::SocketAddr;
use std::sync::Arc;

#[must_use]
pub fn cors_allowed_hosts_with_web_admin_panel(
    cors_allowed_hosts: &[String],
    web_admin_panel_addr: Option<SocketAddr>,
) -> Arc<Vec<String>> {
    Arc::new(
        cors_allowed_hosts
            .iter()
            .cloned()
            .chain(
                web_admin_panel_addr
                    .map(|web_admin_panel_addr| format!("http://{web_admin_panel_addr}")),
            )
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;
    use std::net::SocketAddr;

    use super::cors_allowed_hosts_with_web_admin_panel;

    #[test]
    fn appends_the_web_admin_panel_origin_to_the_configured_hosts() {
        let allowed_hosts = cors_allowed_hosts_with_web_admin_panel(
            &["http://127.0.0.1:8080".to_owned()],
            Some(SocketAddr::from((Ipv4Addr::LOCALHOST, 9000))),
        );

        assert_eq!(
            *allowed_hosts,
            vec![
                "http://127.0.0.1:8080".to_owned(),
                "http://127.0.0.1:9000".to_owned(),
            ]
        );
    }

    #[test]
    fn keeps_only_the_configured_hosts_without_a_web_admin_panel() {
        let allowed_hosts =
            cors_allowed_hosts_with_web_admin_panel(&["http://127.0.0.1:8080".to_owned()], None);

        assert_eq!(*allowed_hosts, vec!["http://127.0.0.1:8080".to_owned()]);
    }
}
