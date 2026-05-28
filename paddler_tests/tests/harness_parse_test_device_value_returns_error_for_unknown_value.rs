use paddler_tests::parse_test_device_value::parse_test_device_value;

#[test]
fn harness_parse_test_device_value_returns_error_for_unknown_value() {
    let result = parse_test_device_value(Some("vulkan"));

    assert!(
        result.is_err(),
        "parse_test_device_value should reject unknown values"
    );
}
