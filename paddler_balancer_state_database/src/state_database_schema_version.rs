use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum StateDatabaseSchemaVersion {
    #[serde(rename = "2")]
    V2,
    #[default]
    #[serde(rename = "3")]
    V3,
}
