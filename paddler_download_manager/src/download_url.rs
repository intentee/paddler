use url::Url;

use crate::download_error::DownloadError;

#[derive(Debug, Eq, PartialEq)]
pub struct DownloadUrl {
    url: Url,
}

impl DownloadUrl {
    pub fn parse(url: &str) -> Result<Self, DownloadError> {
        Self::try_from(Url::parse(url).map_err(|source| DownloadError::InvalidUrl {
            url: url.to_owned(),
            source,
        })?)
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }
}

impl TryFrom<Url> for DownloadUrl {
    type Error = DownloadError;

    fn try_from(url: Url) -> Result<Self, DownloadError> {
        match url.scheme() {
            "http" | "https" => Ok(Self { url }),
            unsupported_scheme => Err(DownloadError::UnsupportedUrlScheme {
                scheme: unsupported_scheme.to_owned(),
                url: url.to_string(),
            }),
        }
    }
}
