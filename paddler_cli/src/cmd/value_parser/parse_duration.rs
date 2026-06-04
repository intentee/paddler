use std::time::Duration;

use anyhow::Result;

pub fn parse_duration(arg: &str) -> Result<Duration> {
    let milliseconds = arg.parse()?;

    Ok(std::time::Duration::from_millis(milliseconds))
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::parse_duration;

    #[test]
    fn parses_milliseconds() {
        assert_eq!(parse_duration("1500").unwrap(), Duration::from_millis(1500));
    }

    #[test]
    fn rejects_a_non_numeric_value() {
        assert!(parse_duration("not-a-number").is_err());
    }
}
