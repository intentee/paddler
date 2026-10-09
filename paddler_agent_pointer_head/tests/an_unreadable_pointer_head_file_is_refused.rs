use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

#[test]
fn an_unreadable_pointer_head_file_is_refused() {
    let invalid_file = fixture_path("invalid.gguf");

    assert!(matches!(
        PointerHead::load(&invalid_file),
        Err(PointerHeadError::FileUnreadable { path, .. }) if path == invalid_file
    ));
}
