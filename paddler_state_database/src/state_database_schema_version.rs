use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum StateDatabaseSchemaVersion {
    #[default]
    #[serde(rename = "1")]
    V1,
}
