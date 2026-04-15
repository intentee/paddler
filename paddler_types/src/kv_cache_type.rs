use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum KvCacheType {
    F16,
    Q4_0,
    Q8_0,
}
