use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

#[test]
fn a_pointer_head_with_f16_tensors_is_refused() {
    assert!(matches!(
        PointerHead::load(&fixture_path("pointer_head_with_f16_tensors.gguf")),
        Err(PointerHeadError::TensorUnreadable { tensor, .. }) if tensor == "pointer_head.key.bias"
    ));
}
