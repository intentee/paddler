use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Error {
    pub code: i32,
    pub description: String,
}

impl Display for Error {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "jsonrpc_error(code={})", self.code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_only_code_ignoring_description() {
        let error = Error {
            code: -32_600,
            description: "Invalid Request".to_owned(),
        };

        assert_eq!("jsonrpc_error(code=-32600)", error.to_string());
    }
}
