use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

#[test]
fn a_pointer_head_without_its_temperature_is_refused() {
    assert!(matches!(
        PointerHead::load(&fixture_path("pointer_head_without_temperature.gguf")),
        Err(PointerHeadError::KeyUnreadable { key, .. }) if key == "pointer_head.temperature"
    ));
}
