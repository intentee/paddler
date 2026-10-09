use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponsesResponseStatus {
    Completed,
    Failed,
    Incomplete,
    InProgress,
}
