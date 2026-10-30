use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

#[test]
fn a_pointer_head_with_mismatched_projections_is_refused() {
    assert_eq!(
        PointerHead::load(&fixture_path(
            "pointer_head_with_mismatched_projections.gguf"
        ))
        .err()
        .as_ref()
        .map(ToString::to_string),
        Some(
            PointerHeadError::TensorShapeMismatch {
                tensor: "pointer_head.query.bias".to_owned(),
                expected: [16, 1, 1, 1],
                actual: [15, 1, 1, 1],
            }
            .to_string()
        )
    );
}
