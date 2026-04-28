#![cfg(feature = "cuda")]

use anyhow::Result;
use paddler_tests::parse_test_device_value::parse_test_device_value;
use paddler_tests::test_device::TestDevice;

#[test]
fn harness_parse_test_device_value_reads_cuda() -> Result<()> {
    assert_eq!(parse_test_device_value(Some("cuda"))?, TestDevice::Cuda);

    Ok(())
}
