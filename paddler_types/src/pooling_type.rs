use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[repr(i8)]
pub enum PoolingType {
    Unspecified = -1,
    None = 0,
    Mean = 1,
    Cls = 2,
    Last = 3,
    Rank = 4,
}
