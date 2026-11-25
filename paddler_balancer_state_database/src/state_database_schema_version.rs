use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum StateDatabaseSchemaVersion {
    #[serde(rename = "2")]
    V2,
    #[serde(rename = "3")]
    V3,
    #[default]
    #[serde(rename = "4")]
    V4,
}
