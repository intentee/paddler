use anyhow::Result;
use paddler_tests::parse_test_device_value::parse_test_device_value;
use paddler_tests::test_device::TestDevice;

#[test]
fn harness_parse_test_device_value_returns_cpu_for_none() -> Result<()> {
    assert_eq!(parse_test_device_value(None)?, TestDevice::Cpu);

    Ok(())
}
