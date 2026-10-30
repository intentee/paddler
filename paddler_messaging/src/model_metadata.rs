use std::collections::BTreeMap;

use serde::Deserialize;
use serde::Serialize;

use crate::chat_template::ChatTemplate;
use crate::embedded_chat_template_key::EMBEDDED_CHAT_TEMPLATE_KEY;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModelMetadata {
    pub metadata: BTreeMap<String, String>,
}

impl ModelMetadata {
    #[must_use]
    pub fn embedded_chat_template(&self) -> Option<ChatTemplate> {
        self.metadata
            .get(EMBEDDED_CHAT_TEMPLATE_KEY)
            .map(|content| ChatTemplate {
                content: content.clone(),
            })
    }

    pub fn set_meta_field(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
}
