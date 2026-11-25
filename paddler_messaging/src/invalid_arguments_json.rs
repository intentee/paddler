use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct InvalidArgumentsJson {
    pub message: String,
    pub line: usize,
    pub column: usize,
}
