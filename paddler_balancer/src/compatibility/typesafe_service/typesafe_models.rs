use serde::Serialize;

use paddler_messaging::agent_desired_model::AgentDesiredModel;
use paddler_messaging::url_model_reference::UrlModelReference;

use crate::compatibility::typesafe_service::typesafe_model::TypeSafeModel;

#[derive(Serialize)]
pub struct TypeSafeModels {
    pub models: Vec<TypeSafeModel>,
}

impl From<AgentDesiredModel> for TypeSafeModels {
    fn from(model: AgentDesiredModel) -> Self {
        Self {
            models: match model {
                AgentDesiredModel::HuggingFace(reference) => {
                    vec![TypeSafeModel::named(reference.model_path().model_path)]
                }
                AgentDesiredModel::LocalToAgent(model_path) => {
                    vec![TypeSafeModel::named(model_path)]
                }
                AgentDesiredModel::Url(UrlModelReference { url }) => {
                    vec![TypeSafeModel::named(url)]
                }
                AgentDesiredModel::None => Vec::new(),
            },
        }
    }
}
