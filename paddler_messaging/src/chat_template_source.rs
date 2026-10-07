use serde::Deserialize;
use serde::Serialize;

use crate::chat_template::ChatTemplate;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub enum ChatTemplateSource {
    #[default]
    EmbeddedInModel,
    Override(ChatTemplate),
}
