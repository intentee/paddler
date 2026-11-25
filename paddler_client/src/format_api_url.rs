use url::Url;

#[must_use]
pub fn format_api_url(base_url: &Url, path: &str) -> String {
    format!("{}{}", base_url.as_str().trim_end_matches('/'), path)
}

#[cfg(test)]
mod tests {
    use url::Url;

    use paddler_messaging::constants::API_PATH_AGENTS;

    use super::format_api_url;

    #[test]
    fn joins_the_path_onto_the_trimmed_base() {
        let base_url = Url::parse("http://localhost:8080/").unwrap();

        assert_eq!(
            format_api_url(&base_url, API_PATH_AGENTS),
            format!("http://localhost:8080{API_PATH_AGENTS}")
        );
    }
}
