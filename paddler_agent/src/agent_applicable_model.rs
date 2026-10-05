use std::path::PathBuf;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum AgentApplicableModel {
    #[default]
    NotConfigured,
    Resolved {
        model_path: PathBuf,
        multimodal_projection_path: Option<PathBuf>,
    },
}
