use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelMetadata {
    pub metadata: BTreeMap<String, String>,
}

impl ModelMetadata {
    pub fn set_meta_field(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}
