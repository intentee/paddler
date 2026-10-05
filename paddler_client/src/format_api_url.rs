use url::Url;

pub fn format_api_url(base_url: &Url, path: &str) -> String {
    format!("{}{}", base_url.as_str().trim_end_matches('/'), path)
}

#[cfg(test)]
mod tests {
    use url::Url;

    use paddler_messaging::api_path::ApiPath;

    use super::format_api_url;

    #[test]
    fn joins_the_path_onto_the_trimmed_base() {
        let base_url = Url::parse("http://localhost:8080/").unwrap();

        assert_eq!(
            format_api_url(&base_url, ApiPath::AGENTS),
            format!("http://localhost:8080{}", ApiPath::AGENTS)
        );
    }
}
