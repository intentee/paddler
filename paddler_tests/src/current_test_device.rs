use std::env;

use anyhow::Result;
use anyhow::bail;

use crate::parse_test_device_value::parse_test_device_value;
use crate::test_device::TestDevice;

const PADDLER_TEST_DEVICE: &str = "PADDLER_TEST_DEVICE";

pub fn current_test_device() -> Result<TestDevice> {
    match env::var(PADDLER_TEST_DEVICE) {
        Ok(value) => parse_test_device_value(Some(&value)),
        Err(env::VarError::NotPresent) => parse_test_device_value(None),
        Err(env::VarError::NotUnicode(value)) => bail!(
            "{PADDLER_TEST_DEVICE} is set but is not valid UTF-8: {}",
            value.to_string_lossy()
        ),
    }
}
