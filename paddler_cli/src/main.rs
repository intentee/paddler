mod fatal_cli_error;

use paddler_cli::run;

use crate::fatal_cli_error::FatalCliError;

fn main() -> Result<(), FatalCliError> {
    Ok(run()?)
}
