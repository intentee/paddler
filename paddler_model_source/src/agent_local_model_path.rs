use std::os::unix::ffi::OsStrExt as _;
use std::path::Component;
use std::path::Path;
use std::path::PathBuf;

use percent_encoding::percent_encode;
use url::Url;

use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
use paddler_messaging::agent_issue::AgentIssue;
use paddler_messaging::agent_issue_params::agent_model_uri_not_canonical_params::AgentModelUriNotCanonicalParams;
use paddler_messaging::agent_issue_params::model_path::ModelPath;

use crate::model_source_error::ModelSourceError;
use crate::url_path_segment_encode_set::URL_PATH_SEGMENT_ENCODE_SET;

fn canonical_agent_uri(normalized_absolute_path: &Path) -> String {
    let mut encoded_path = String::new();

    for component in normalized_absolute_path.components().skip(1) {
        encoded_path.push('/');
        encoded_path.extend(percent_encode(
            component.as_os_str().as_bytes(),
            URL_PATH_SEGMENT_ENCODE_SET,
        ));
    }

    if encoded_path.is_empty() {
        format!("{}:///", AgentLocalModelPath::SCHEME)
    } else {
        format!("{}://{encoded_path}", AgentLocalModelPath::SCHEME)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentLocalModelPath {
    canonical_uri: String,
    path: PathBuf,
}

impl AgentLocalModelPath {
    pub const SCHEME: &str = "agent";

    pub fn from_absolute_path(path: &Path) -> Result<Self, ModelSourceError> {
        if !path.is_absolute() {
            return Err(ModelSourceError::LocalModelPathNotAbsolute {
                path: path.to_path_buf(),
            });
        }

        if path
            .components()
            .any(|component| component == Component::ParentDir)
        {
            return Err(ModelSourceError::LocalModelPathClimbsToParentDirectory {
                path: path.to_path_buf(),
            });
        }

        Ok(Self::from_normalized_absolute_path(
            path.components().collect(),
        ))
    }

    pub fn from_agent_url(
        uri: &str,
        agent_url: &Url,
        slot_aggregated_status: &SlotAggregatedStatus,
    ) -> Result<Self, ModelSourceError> {
        let model_path = || ModelPath {
            model_path: uri.to_owned(),
        };
        let path = agent_url.to_file_path().map_err(|()| {
            slot_aggregated_status.register_desired_state_issue(
                AgentIssue::AgentModelUriIsNotAnAbsoluteLocalPath(model_path()),
            );

            ModelSourceError::AgentUriNotAnAbsoluteLocalPath {
                uri: uri.to_owned(),
            }
        })?;
        let agent_local_model_path = Self::from_normalized_absolute_path(path);

        if agent_local_model_path.canonical_uri != uri {
            slot_aggregated_status.register_desired_state_issue(
                AgentIssue::AgentModelUriIsNotCanonical(AgentModelUriNotCanonicalParams {
                    canonical_uri: agent_local_model_path.canonical_uri.clone(),
                    model_path: model_path(),
                }),
            );

            return Err(ModelSourceError::AgentUriNotCanonical {
                canonical_uri: agent_local_model_path.canonical_uri,
                uri: uri.to_owned(),
            });
        }

        Ok(agent_local_model_path)
    }

    #[must_use]
    pub fn canonical_uri(&self) -> &str {
        &self.canonical_uri
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    fn from_normalized_absolute_path(path: PathBuf) -> Self {
        Self {
            canonical_uri: canonical_agent_uri(&path),
            path,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use url::Url;

    use paddler_agent_status::slot_aggregated_status::SlotAggregatedStatus;
    use paddler_messaging::agent_issue::AgentIssue;
    use paddler_messaging::agent_issue_params::agent_model_uri_not_canonical_params::AgentModelUriNotCanonicalParams;
    use paddler_messaging::agent_issue_params::model_path::ModelPath;

    use super::AgentLocalModelPath;
    use crate::model_source_error::ModelSourceError;

    fn parsed(uri: &str, slot_aggregated_status: &SlotAggregatedStatus) -> AgentLocalModelPath {
        AgentLocalModelPath::from_agent_url(uri, &Url::parse(uri).unwrap(), slot_aggregated_status)
            .unwrap()
    }

    fn refusal(uri: &str, slot_aggregated_status: &SlotAggregatedStatus) -> String {
        AgentLocalModelPath::from_agent_url(uri, &Url::parse(uri).unwrap(), slot_aggregated_status)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn formats_a_path_with_reserved_and_non_ascii_characters_as_its_canonical_uri() {
        assert_eq!(
            AgentLocalModelPath::from_absolute_path(Path::new("/models/my mod\u{e8}l #1?%.gguf"))
                .unwrap()
                .canonical_uri(),
            "agent:///models/my%20mod%C3%A8l%20%231%3F%25.gguf"
        );
    }

    #[test]
    fn formats_the_root_directory_as_the_canonical_root_uri() {
        assert_eq!(
            AgentLocalModelPath::from_absolute_path(Path::new("/"))
                .unwrap()
                .canonical_uri(),
            "agent:///"
        );
    }

    #[test]
    fn decodes_a_canonical_uri_into_the_path_it_names() {
        assert_eq!(
            parsed(
                "agent:///models/my%20mod%C3%A8l%20%231%3F%25.gguf",
                &SlotAggregatedStatus::new(1)
            )
            .path(),
            Path::new("/models/my mod\u{e8}l #1?%.gguf")
        );
    }

    #[test]
    fn refuses_to_format_a_relative_path() {
        assert_eq!(
            AgentLocalModelPath::from_absolute_path(Path::new("models/model.gguf"))
                .unwrap_err()
                .to_string(),
            ModelSourceError::LocalModelPathNotAbsolute {
                path: "models/model.gguf".into(),
            }
            .to_string()
        );
    }

    #[test]
    fn refuses_to_format_a_path_that_climbs_to_a_parent_directory() {
        assert_eq!(
            AgentLocalModelPath::from_absolute_path(Path::new("/models/../model.gguf"))
                .unwrap_err()
                .to_string(),
            ModelSourceError::LocalModelPathClimbsToParentDirectory {
                path: "/models/../model.gguf".into(),
            }
            .to_string()
        );
    }

    #[test]
    fn refuses_every_spelling_that_differs_from_the_canonical_uri() {
        for (uri, canonical_uri) in [
            (
                "agent:///models/my model.gguf",
                "agent:///models/my%20model.gguf",
            ),
            (
                "agent:///models/mod\u{e8}l.gguf",
                "agent:///models/mod%C3%A8l.gguf",
            ),
            ("agent:///models/%6Dodel.gguf", "agent:///models/model.gguf"),
            ("agent:///models/a%2Fb.gguf", "agent:///models/a/b.gguf"),
            ("agent:///models/../model.gguf", "agent:///model.gguf"),
            ("agent:///models//model.gguf", "agent:///models/model.gguf"),
            (
                "agent:///models/model.gguf?revision=1",
                "agent:///models/model.gguf",
            ),
            (
                "agent:///models/model.gguf#fragment",
                "agent:///models/model.gguf",
            ),
            (
                "agent://localhost/models/model.gguf",
                "agent:///models/model.gguf",
            ),
            ("agent:/models/model.gguf", "agent:///models/model.gguf"),
            ("AGENT:///models/model.gguf", "agent:///models/model.gguf"),
        ] {
            let slot_aggregated_status = SlotAggregatedStatus::new(1);

            assert_eq!(
                refusal(uri, &slot_aggregated_status),
                ModelSourceError::AgentUriNotCanonical {
                    canonical_uri: canonical_uri.to_owned(),
                    uri: uri.to_owned(),
                }
                .to_string()
            );
            assert!(
                slot_aggregated_status.has_issue(&AgentIssue::AgentModelUriIsNotCanonical(
                    AgentModelUriNotCanonicalParams {
                        canonical_uri: canonical_uri.to_owned(),
                        model_path: ModelPath {
                            model_path: uri.to_owned(),
                        },
                    }
                ))
            );
        }
    }

    #[test]
    fn refuses_uris_that_do_not_name_an_absolute_local_path() {
        for uri in ["agent://models/model.gguf", "agent:model.gguf"] {
            let slot_aggregated_status = SlotAggregatedStatus::new(1);

            assert_eq!(
                refusal(uri, &slot_aggregated_status),
                ModelSourceError::AgentUriNotAnAbsoluteLocalPath {
                    uri: uri.to_owned(),
                }
                .to_string()
            );
            assert!(slot_aggregated_status.has_issue(
                &AgentIssue::AgentModelUriIsNotAnAbsoluteLocalPath(ModelPath {
                    model_path: uri.to_owned(),
                })
            ));
        }
    }
}
