use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum ModelDownloadStatus {
    Downloading {
        downloaded_bytes: u64,
        model_path: String,
        total_bytes: u64,
    },
    DownloadingWithUnknownSize {
        downloaded_bytes: u64,
        model_path: String,
    },
    #[default]
    NotDownloading,
}

impl ModelDownloadStatus {
    pub const fn add_downloaded_bytes(&mut self, added_bytes: u64) {
        match self {
            Self::Downloading {
                downloaded_bytes, ..
            }
            | Self::DownloadingWithUnknownSize {
                downloaded_bytes, ..
            } => *downloaded_bytes += added_bytes,
            Self::NotDownloading => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ModelDownloadStatus;

    #[test]
    fn adds_downloaded_bytes_to_a_running_download() {
        let mut download_status = ModelDownloadStatus::DownloadingWithUnknownSize {
            downloaded_bytes: 100,
            model_path: "https://example.com/model.gguf".to_owned(),
        };

        download_status.add_downloaded_bytes(50);

        assert_eq!(
            download_status,
            ModelDownloadStatus::DownloadingWithUnknownSize {
                downloaded_bytes: 150,
                model_path: "https://example.com/model.gguf".to_owned(),
            }
        );
    }

    #[test]
    fn bytes_arriving_after_the_download_stopped_do_not_restart_it() {
        let mut download_status = ModelDownloadStatus::NotDownloading;

        download_status.add_downloaded_bytes(50);

        assert_eq!(download_status, ModelDownloadStatus::NotDownloading);
    }
}
