use std::time::Duration;

use anyhow::Result;

pub fn parse_duration(arg: &str) -> Result<Duration> {
    let milliseconds = arg.parse()?;

    Ok(std::time::Duration::from_millis(milliseconds))
}
