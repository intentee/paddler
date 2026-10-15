use std::error::Error;

use llama_cpp_bindings::gguf_context_error::GgufContextError;

use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

#[test]
fn an_unreadable_pointer_head_file_is_refused() {
    let invalid_file = fixture_path("invalid.gguf");

    let load_error = PointerHead::load(&invalid_file).err();

    assert_eq!(
        load_error.as_ref().map(ToString::to_string),
        Some(
            PointerHeadError::FileUnreadable {
                path: invalid_file.clone(),
                source: GgufContextError::InitFailed(invalid_file.clone()),
            }
            .to_string()
        )
    );
    assert_eq!(
        load_error
            .as_ref()
            .and_then(Error::source)
            .and_then(|source| source.downcast_ref::<GgufContextError>())
            .map(ToString::to_string),
        Some(GgufContextError::InitFailed(invalid_file).to_string())
    );
}
