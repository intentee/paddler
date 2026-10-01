use url::Url;

use crate::download_error::DownloadError;

pub struct DownloadUrl {
    url: Url,
}

impl DownloadUrl {
    pub fn parse(url: &str) -> Result<Self, DownloadError> {
        let parsed_url = Url::parse(url).map_err(|source| DownloadError::InvalidUrl {
            url: url.to_owned(),
            source,
        })?;

        match parsed_url.scheme() {
            "http" | "https" => Ok(Self { url: parsed_url }),
            unsupported_scheme => Err(DownloadError::UnsupportedUrlScheme {
                url: url.to_owned(),
                scheme: unsupported_scheme.to_owned(),
            }),
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }
}
