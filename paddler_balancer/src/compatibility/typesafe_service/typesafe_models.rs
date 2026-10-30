use std::collections::BTreeSet;

use serde::Serialize;

use crate::compatibility::typesafe_service::typesafe_model::TypeSafeModel;

#[derive(Serialize)]
pub struct TypeSafeModels {
    pub models: Vec<TypeSafeModel>,
}

impl From<BTreeSet<String>> for TypeSafeModels {
    fn from(served_model_paths: BTreeSet<String>) -> Self {
        Self {
            models: served_model_paths
                .into_iter()
                .map(TypeSafeModel::named)
                .collect(),
        }
    }
}
