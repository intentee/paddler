#![cfg(not(feature = "metal"))]

use paddler_tests::parse_test_device_value::parse_test_device_value;

#[test]
fn harness_parse_test_device_value_rejects_metal_when_feature_is_not_linked() {
    let result = parse_test_device_value(Some("metal"));

    assert!(
        result.is_err(),
        "PADDLER_TEST_DEVICE=metal must be rejected when the metal backend is not linked"
    );
}
