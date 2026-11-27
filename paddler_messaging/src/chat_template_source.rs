use serde::Deserialize;
use serde::Serialize;

use crate::chat_template::ChatTemplate;

#[derive(Clone, Debug, Default, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub enum ChatTemplateSource {
    #[default]
    EmbeddedInModel,
    Override(ChatTemplate),
}
