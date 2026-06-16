use anyhow::Context as _;
use anyhow::Result;
use async_openai::config::OpenAIConfig;
use url::Url;

pub fn openai_config_from_base_url(openai_base_url: &Url) -> Result<OpenAIConfig> {
    let api_base = openai_base_url
        .join("v1")
        .context("failed to build the OpenAI /v1 base URL")?;

    Ok(OpenAIConfig::default()
        .with_api_base(api_base.as_str().trim_end_matches('/'))
        .with_api_key("paddler"))
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::openai_config_from_base_url;

    #[test]
    fn builds_a_v1_api_base_from_the_root_url() {
        let config =
            openai_config_from_base_url(&Url::parse("http://127.0.0.1:8062/").unwrap()).unwrap();

        assert_eq!(
            async_openai::config::Config::api_base(&config),
            "http://127.0.0.1:8062/v1"
        );
    }

    #[test]
    fn errors_for_an_unbuildable_base_url() {
        let error = openai_config_from_base_url(&Url::parse("data:text/plain,paddler").unwrap())
            .err()
            .unwrap();

        assert!(error.to_string().contains("/v1 base URL"));
    }
}
