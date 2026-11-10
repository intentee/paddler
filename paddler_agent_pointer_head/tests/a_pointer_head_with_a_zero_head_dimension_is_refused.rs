use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;
use paddler_test_cluster_harness::fixture_path::fixture_path;

#[test]
fn a_pointer_head_with_a_zero_head_dimension_is_refused() {
    assert_eq!(
        PointerHead::load(&fixture_path("pointer_head_with_zero_head_dim.gguf"))
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(PointerHeadError::HeadDimensionZero.to_string())
    );
}
