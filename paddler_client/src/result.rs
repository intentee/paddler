use std::result::Result as StdResult;

use crate::error::Error;

pub type Result<TValue> = StdResult<TValue, Error>;
