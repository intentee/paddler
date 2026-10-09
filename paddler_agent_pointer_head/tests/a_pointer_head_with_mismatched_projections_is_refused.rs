use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

#[test]
fn a_pointer_head_with_mismatched_projections_is_refused() {
    assert!(matches!(
        PointerHead::load(&fixture_path("pointer_head_with_mismatched_projections.gguf")),
        Err(PointerHeadError::TensorShapeMismatch {
            tensor,
            expected: [16, 1, 1, 1],
            actual: [15, 1, 1, 1],
        }) if tensor == "pointer_head.query.bias"
    ));
}
