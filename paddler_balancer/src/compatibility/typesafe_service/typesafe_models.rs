use serde::Serialize;

use paddler_messaging::agent_desired_model::AgentDesiredModel;

use crate::compatibility::typesafe_service::typesafe_model::TypeSafeModel;

#[derive(Serialize)]
pub struct TypeSafeModels {
    pub models: Vec<TypeSafeModel>,
}

impl From<AgentDesiredModel> for TypeSafeModels {
    fn from(model: AgentDesiredModel) -> Self {
        Self {
            models: match model {
                AgentDesiredModel::None => Vec::new(),
                AgentDesiredModel::Uri(uri) => vec![TypeSafeModel::named(uri)],
            },
        }
    }
}
