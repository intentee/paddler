use std::fmt;
use std::fmt::Debug;
use std::fmt::Formatter;

use thiserror::Error;

use paddler_cli::cli_error::CliError;
use paddler_error_chain::error_chain::ErrorChain;

#[derive(Error)]
pub enum FatalCliError {
    #[error(transparent)]
    Cli(#[from] CliError),
}

impl Debug for FatalCliError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", ErrorChain { error: self })
    }
}
