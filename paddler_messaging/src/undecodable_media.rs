use serde::Deserialize;
use serde::Serialize;

use crate::media_kind::MediaKind;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UndecodableMedia {
    pub description: String,
    pub media: MediaKind,
}
