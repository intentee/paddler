#![cfg(not(feature = "cuda"))]

use paddler_tests::parse_test_device_value::parse_test_device_value;

#[test]
fn harness_parse_test_device_value_rejects_cuda_when_feature_is_not_linked() {
    let result = parse_test_device_value(Some("cuda"));

    assert!(
        result.is_err(),
        "PADDLER_TEST_DEVICE=cuda must be rejected when the cuda backend is not linked"
    );
}
