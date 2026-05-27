#![cfg(feature = "metal")]

use anyhow::Result;
use paddler_cli_tests::parse_test_device_value::parse_test_device_value;
use paddler_cli_tests::test_device::TestDevice;

#[test]
fn harness_parse_test_device_value_reads_metal() -> Result<()> {
    assert_eq!(parse_test_device_value(Some("metal"))?, TestDevice::Metal);

    Ok(())
}
