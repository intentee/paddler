use percent_encoding::percent_decode_str;
use percent_encoding::percent_encode;
use url::Url;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_download_manager::download_url::DownloadUrl;
use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::agent_local_model_path::AgentLocalModelPath;
use crate::download_error_agent_issue::download_error_agent_issue;
use crate::huggingface_model_reference::HuggingFaceModelReference;
use crate::model_source_error::ModelSourceError;
use crate::url_path_segment_encode_set::URL_PATH_SEGMENT_ENCODE_SET;

const HUGGING_FACE_BLOB: &str = "blob";
const HUGGING_FACE_HOST: &str = "huggingface.co";
const HUGGING_FACE_RESOLVE: &str = "resolve";

fn huggingface_model_reference(
    uri: &str,
    url: &Url,
) -> Result<HuggingFaceModelReference, ModelSourceError> {
    let path_segments = url
        .path()
        .split('/')
        .filter(|path_segment| !path_segment.is_empty())
        .map(|path_segment| {
            percent_decode_str(path_segment)
                .decode_utf8()
                .map_err(
                    |source| ModelSourceError::HuggingFaceModelUriComponentNotUtf8 {
                        source,
                        uri: uri.to_owned(),
                    },
                )
        })
        .collect::<Result<Vec<_>, _>>()?;

    match path_segments.as_slice() {
        [owner, repo, file_kind, revision, filename_segments @ ..]
            if (file_kind == HUGGING_FACE_BLOB || file_kind == HUGGING_FACE_RESOLVE)
                && !filename_segments.is_empty() =>
        {
            Ok(HuggingFaceModelReference {
                filename: filename_segments.join("/"),
                repository_name: repo.to_string(),
                repository_owner: owner.to_string(),
                revision: revision.to_string(),
            })
        }
        _ => Err(ModelSourceError::HuggingFaceModelUriMalformed {
            uri: uri.to_owned(),
        }),
    }
}

fn percent_encoded_path_segment(path_segment: &str) -> String {
    percent_encode(path_segment.as_bytes(), URL_PATH_SEGMENT_ENCODE_SET).to_string()
}

fn percent_encoded_path(path: &str) -> String {
    path.split('/')
        .map(percent_encoded_path_segment)
        .collect::<Vec<_>>()
        .join("/")
}

#[derive(Debug, Eq, PartialEq)]
pub enum ModelSource {
    HuggingFace(HuggingFaceModelReference),
    LocalToAgent(AgentLocalModelPath),
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
            slot_aggregated_status
                .register_desired_state_issue(AgentIssue::ModelUriIsUnparseable(model_path()));

            ModelSourceError::ModelUriUnparseable {
                source,
                uri: uri.to_owned(),
            }
        })?;

        if url.host_str() == Some(HUGGING_FACE_HOST) {
            return huggingface_model_reference(uri, &url)
                .map(Self::HuggingFace)
                .inspect_err(|_malformed_uri| {
                    slot_aggregated_status.register_desired_state_issue(
                        AgentIssue::HuggingFaceModelUriIsMalformed(model_path()),
                    );
                });
        }

        if url.scheme() == AgentLocalModelPath::SCHEME {
            return AgentLocalModelPath::from_agent_url(uri, &url, slot_aggregated_status)
                .map(Self::LocalToAgent);
        }

        DownloadUrl::try_from(url)
            .map(Self::Url)
            .map_err(|download_error| {
                slot_aggregated_status.register_desired_state_issue(download_error_agent_issue(
                    &download_error,
                    model_path(),
                ));

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
                repository_name,
                repository_owner,
                revision,
            }) => format!(
                "https://{HUGGING_FACE_HOST}/{}/{}/{HUGGING_FACE_BLOB}/{}/{}",
                percent_encoded_path_segment(repository_owner),
                percent_encoded_path_segment(repository_name),
                percent_encoded_path_segment(revision),
                percent_encoded_path(filename)
            ),
            Self::LocalToAgent(agent_local_model_path) => {
                agent_local_model_path.canonical_uri().to_owned()
            }
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
    use std::error::Error as _;
    use std::path::Path;

    use url::ParseError;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_download_manager::download_error::DownloadError;
    use paddler_download_manager::download_url::DownloadUrl;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;

    use super::ModelSource;
    use crate::agent_local_model_path::AgentLocalModelPath;
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
                    repository_name: "Qwen3-0.6B-GGUF".to_owned(),
                    repository_owner: "unsloth".to_owned(),
                    revision: "main".to_owned(),
                })
            );
        }
    }

    #[test]
    fn decodes_the_percent_encoded_components_of_hugging_face_uris() {
        let slot_aggregated_status = SlotAggregatedStatus::new(1);

        for (uri, filename, revision) in [
            (
                "https://huggingface.co/owner/repo/resolve/main/my%20model.gguf",
                "my model.gguf",
                "main",
            ),
            (
                "https://huggingface.co/owner/repo/blob/main/nested/mod%C3%A8le.gguf",
                "nested/modèle.gguf",
                "main",
            ),
            (
                "https://huggingface.co/owner/repo/resolve/refs%2Fpr%2F1/model.gguf",
                "model.gguf",
                "refs/pr/1",
            ),
        ] {
            assert_eq!(
                ModelSource::parse(uri, &slot_aggregated_status).unwrap(),
                ModelSource::HuggingFace(HuggingFaceModelReference {
                    filename: filename.to_owned(),
                    repository_name: "repo".to_owned(),
                    repository_owner: "owner".to_owned(),
                    revision: revision.to_owned(),
                })
            );
        }
    }

    #[test]
    fn rejects_hugging_face_uris_whose_components_do_not_decode_to_utf8() {
        let slot_aggregated_status = SlotAggregatedStatus::new(1);
        let uri = "https://huggingface.co/owner/repo/blob/main/model%FF.gguf";
        let parse_error = ModelSource::parse(uri, &slot_aggregated_status)
            .expect_err("a component that is not utf-8 must be rejected");

        assert_eq!(
            (
                parse_error.to_string(),
                parse_error.source().map(ToString::to_string)
            ),
            (
                format!(
                    "Hugging Face model URI '{uri}' has a path component that is not valid UTF-8 once percent-decoded: invalid utf-8 sequence of 1 bytes from index 5"
                ),
                Some("invalid utf-8 sequence of 1 bytes from index 5".to_owned())
            )
        );
        assert!(
            slot_aggregated_status
                .has_issue(&AgentIssue::HuggingFaceModelUriIsMalformed(model_path(uri)))
        );
    }

    #[test]
    fn rejects_hugging_face_uris_that_do_not_point_at_a_file() {
        for uri in [
            "https://huggingface.co/owner/repo",
            "https://huggingface.co/owner/repo/tree/main/model.gguf",
            "https://huggingface.co/owner/repo/blob/main",
        ] {
            let slot_aggregated_status = SlotAggregatedStatus::new(1);

            assert_eq!(
                ModelSource::parse(uri, &slot_aggregated_status)
                    .err()
                    .as_ref()
                    .map(ToString::to_string),
                Some(
                    ModelSourceError::HuggingFaceModelUriMalformed {
                        uri: uri.to_owned(),
                    }
                    .to_string()
                )
            );
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
            ModelSource::LocalToAgent(
                AgentLocalModelPath::from_absolute_path(Path::new("/models/model.gguf")).unwrap()
            )
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
                    filename: "nested/my model.gguf".to_owned(),
                    repository_name: "repo".to_owned(),
                    repository_owner: "owner".to_owned(),
                    revision: "refs/pr/1".to_owned(),
                }),
                ModelSource::LocalToAgent(
                    AgentLocalModelPath::from_absolute_path(Path::new("/models/model.gguf"))
                        .unwrap(),
                ),
                ModelSource::Url(DownloadUrl::parse("https://example.com/model.gguf").unwrap()),
            ]
            .iter()
            .map(ModelSource::uri)
            .collect::<Vec<_>>(),
            [
                "https://huggingface.co/owner/repo/blob/refs%2Fpr%2F1/nested/my%20model.gguf",
                "agent:///models/model.gguf",
                "https://example.com/model.gguf",
            ]
        );
    }

    #[test]
    fn rejects_an_unparseable_uri() {
        let slot_aggregated_status = SlotAggregatedStatus::new(1);
        let parse_error = ModelSource::parse("not a uri", &slot_aggregated_status)
            .expect_err("a uri that is not a url must be rejected");

        assert_eq!(
            parse_error.to_string(),
            ModelSourceError::ModelUriUnparseable {
                source: ParseError::RelativeUrlWithoutBase,
                uri: "not a uri".to_owned(),
            }
            .to_string()
        );
        assert_eq!(
            parse_error
                .source()
                .and_then(|source| source.downcast_ref::<ParseError>()),
            Some(&ParseError::RelativeUrlWithoutBase)
        );
        assert!(
            slot_aggregated_status
                .has_issue(&AgentIssue::ModelUriIsUnparseable(model_path("not a uri")))
        );
    }

    #[test]
    fn rejects_a_uri_that_cannot_be_downloaded() {
        let slot_aggregated_status = SlotAggregatedStatus::new(1);
        let uri = "ftp://example.com/model.gguf";
        let unsupported_scheme = || DownloadError::UnsupportedUrlScheme {
            scheme: "ftp".to_owned(),
            url: uri.to_owned(),
        };
        let parse_error = ModelSource::parse(uri, &slot_aggregated_status)
            .expect_err("a uri with a scheme that cannot be downloaded must be rejected");

        assert_eq!(
            parse_error.to_string(),
            ModelSourceError::ModelUriNotDownloadable {
                source: unsupported_scheme(),
                uri: uri.to_owned(),
            }
            .to_string()
        );
        assert_eq!(
            parse_error
                .source()
                .and_then(|source| source.downcast_ref::<DownloadError>())
                .map(ToString::to_string),
            Some(unsupported_scheme().to_string())
        );
        assert!(
            slot_aggregated_status.has_issue(&AgentIssue::DownloadUrlIsMalformed(model_path(uri)))
        );
    }
}
