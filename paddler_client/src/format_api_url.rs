use url::Url;

use crate::error::Error;
use crate::error::Result;

pub fn format_api_url(base_url: &Url, path: &str) -> Result<String> {
    if !path.starts_with('/') {
        return Err(Error::Other(format!("path must start with '/': {path}")));
    }

    Ok(format!(
        "{}{}",
        base_url.as_str().trim_end_matches('/'),
        path,
    ))
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::format_api_url;
    use crate::error::Error;

    #[test]
    fn test_formats_valid_url() -> std::result::Result<(), Error> {
        let base_url = Url::parse("http://localhost:8080")?;

        assert_eq!(
            format_api_url(&base_url, "/api/v1/health")?,
            "http://localhost:8080/api/v1/health"
        );

        Ok(())
    }

    #[test]
    fn test_rejects_path_without_leading_slash() -> std::result::Result<(), Error> {
        let base_url = Url::parse("http://localhost:8080")?;
        let result = format_api_url(&base_url, "api/v1/health");

        assert!(result.is_err());

        Ok(())
    }
}
