use url::Url;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_download_manager::download_url::DownloadUrl;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::download_error_agent_issue::download_error_agent_issue;
use crate::huggingface_model_reference::HuggingFaceModelReference;
use crate::model_source_error::ModelSourceError;

const AGENT_SCHEME: &str = "agent";
const HUGGING_FACE_BLOB: &str = "blob";
const HUGGING_FACE_HOST: &str = "huggingface.co";
const HUGGING_FACE_RESOLVE: &str = "resolve";

fn huggingface_model_reference(url: &Url) -> Option<HuggingFaceModelReference> {
    let path_segments: Vec<&str> = url
        .path()
        .split('/')
        .filter(|path_segment| !path_segment.is_empty())
        .collect();

    match path_segments.as_slice() {
        [owner, repo, file_kind, revision, filename_segments @ ..]
            if (*file_kind == HUGGING_FACE_BLOB || *file_kind == HUGGING_FACE_RESOLVE)
                && !filename_segments.is_empty() =>
        {
            Some(HuggingFaceModelReference {
                filename: filename_segments.join("/"),
                repo_id: format!("{owner}/{repo}"),
                revision: (*revision).to_owned(),
            })
        }
        _ => None,
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ModelSource {
    HuggingFace(HuggingFaceModelReference),
    LocalToAgent(String),
    Url(DownloadUrl),
}

impl ModelSource {
    pub fn parse(
        uri: &str,
        slot_aggregated_status: &SlotAggregatedStatus,
    ) -> Result<Self, ModelSourceError> {
        let model_path = || ModelPath {
            model_path: uri.to_owned(),
        };
        let url = Url::parse(uri).map_err(|source| {
            slot_aggregated_status.register_issue(AgentIssue::ModelUriIsUnparseable(model_path()));

            ModelSourceError::ModelUriUnparseable {
                source,
                uri: uri.to_owned(),
            }
        })?;

        if url.host_str() == Some(HUGGING_FACE_HOST) {
            return huggingface_model_reference(&url)
                .map(Self::HuggingFace)
                .ok_or_else(|| {
                    slot_aggregated_status
                        .register_issue(AgentIssue::HuggingFaceModelUriIsMalformed(model_path()));

                    ModelSourceError::HuggingFaceModelUriMalformed {
                        uri: uri.to_owned(),
                    }
                });
        }

        if url.scheme() == AGENT_SCHEME {
            return Ok(Self::LocalToAgent(url.path().to_owned()));
        }

        DownloadUrl::try_from(url)
            .map(Self::Url)
            .map_err(|download_error| {
                slot_aggregated_status
                    .register_issue(download_error_agent_issue(&download_error, model_path()));

                ModelSourceError::ModelUriNotDownloadable {
                    source: download_error,
                    uri: uri.to_owned(),
                }
            })
    }

    #[must_use]
    pub fn uri(&self) -> String {
        match self {
            Self::HuggingFace(HuggingFaceModelReference {
                filename,
                repo_id,
                revision,
            }) => format!(
                "https://{HUGGING_FACE_HOST}/{repo_id}/{HUGGING_FACE_BLOB}/{revision}/{filename}"
            ),
            Self::LocalToAgent(path) => format!("{AGENT_SCHEME}://{path}"),
            Self::Url(download_url) => download_url.as_str().to_owned(),
        }
    }

    #[must_use]
    pub fn into_agent_desired_model(self) -> AgentDesiredModel {
        AgentDesiredModel::Uri(self.uri())
    }
}

#[cfg(test)]
mod tests {
    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_download_manager::download_error::DownloadError;
    use paddler_download_manager::download_url::DownloadUrl;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;

    use super::ModelSource;
    use crate::huggingface_model_reference::HuggingFaceModelReference;
    use crate::model_source_error::ModelSourceError;

    fn model_path(uri: &str) -> ModelPath {
        ModelPath {
            model_path: uri.to_owned(),
        }
    }

    #[test]
    fn parses_hugging_face_file_uris() {
        let slot_aggregated_status = SlotAggregatedStatus::new(1);

        for (uri, filename) in [
            (
                "https://huggingface.co/unsloth/Qwen3-0.6B-GGUF/blob/main/Qwen3-0.6B-Q4_K_M.gguf",
                "Qwen3-0.6B-Q4_K_M.gguf",
            ),
            (
                "https://huggingface.co/unsloth/Qwen3-0.6B-GGUF/resolve/main/nested/model.gguf",
                "nested/model.gguf",
            ),
        ] {
            assert_eq!(
                ModelSource::parse(uri, &slot_aggregated_status).unwrap(),
                ModelSource::HuggingFace(HuggingFaceModelReference {
                    filename: filename.to_owned(),
                    repo_id: "unsloth/Qwen3-0.6B-GGUF".to_owned(),
                    revision: "main".to_owned(),
                })
            );
        }
    }

    #[test]
    fn rejects_hugging_face_uris_that_do_not_point_at_a_file() {
        for uri in [
            "https://huggingface.co/owner/repo",
            "https://huggingface.co/owner/repo/tree/main/model.gguf",
            "https://huggingface.co/owner/repo/blob/main",
        ] {
            let slot_aggregated_status = SlotAggregatedStatus::new(1);

            assert!(matches!(
                ModelSource::parse(uri, &slot_aggregated_status),
                Err(ModelSourceError::HuggingFaceModelUriMalformed { uri: rejected_uri })
                    if rejected_uri == uri
            ));
            assert!(
                slot_aggregated_status
                    .has_issue(&AgentIssue::HuggingFaceModelUriIsMalformed(model_path(uri)))
            );
        }
    }

    #[test]
    fn parses_an_agent_uri_as_a_path_local_to_the_agent() {
        assert_eq!(
            ModelSource::parse("agent:///models/model.gguf", &SlotAggregatedStatus::new(1))
                .unwrap(),
            ModelSource::LocalToAgent("/models/model.gguf".to_owned())
        );
    }

    #[test]
    fn parses_an_http_uri_as_a_download() {
        assert_eq!(
            ModelSource::parse(
                "https://example.com/models/model.gguf",
                &SlotAggregatedStatus::new(1)
            )
            .unwrap(),
            ModelSource::Url(DownloadUrl::parse("https://example.com/models/model.gguf").unwrap())
        );
    }

    #[test]
    fn formats_each_source_as_its_canonical_uri() {
        assert_eq!(
            [
                ModelSource::HuggingFace(HuggingFaceModelReference {
                    filename: "nested/model.gguf".to_owned(),
                    repo_id: "owner/repo".to_owned(),
                    revision: "main".to_owned(),
                }),
                ModelSource::LocalToAgent("/models/model.gguf".to_owned()),
                ModelSource::Url(DownloadUrl::parse("https://example.com/model.gguf").unwrap()),
            ]
            .iter()
            .map(ModelSource::uri)
            .collect::<Vec<_>>(),
            [
                "https://huggingface.co/owner/repo/blob/main/nested/model.gguf",
                "agent:///models/model.gguf",
                "https://example.com/model.gguf",
            ]
        );
    }

    #[test]
    fn rejects_an_unparseable_uri() {
        let slot_aggregated_status = SlotAggregatedStatus::new(1);

        assert!(matches!(
            ModelSource::parse("not a uri", &slot_aggregated_status),
            Err(ModelSourceError::ModelUriUnparseable { uri, .. }) if uri == "not a uri"
        ));
        assert!(
            slot_aggregated_status
                .has_issue(&AgentIssue::ModelUriIsUnparseable(model_path("not a uri")))
        );
    }

    #[test]
    fn rejects_a_uri_that_cannot_be_downloaded() {
        let slot_aggregated_status = SlotAggregatedStatus::new(1);
        let uri = "ftp://example.com/model.gguf";

        assert!(matches!(
            ModelSource::parse(uri, &slot_aggregated_status),
            Err(ModelSourceError::ModelUriNotDownloadable {
                source: DownloadError::UnsupportedUrlScheme { scheme, .. },
                ..
            }) if scheme == "ftp"
        ));
        assert!(
            slot_aggregated_status.has_issue(&AgentIssue::DownloadUrlIsMalformed(model_path(uri)))
        );
    }
}
