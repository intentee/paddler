use std::num::NonZeroU16;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReservedRequest<TParams> {
    pub params: TParams,
    pub reserved_slots: NonZeroU16,
}
