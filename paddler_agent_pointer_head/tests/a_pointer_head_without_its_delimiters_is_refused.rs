use std::error::Error;

use llama_cpp_bindings::gguf_context_error::GgufContextError;

use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

fn decide_delimiter_not_found() -> GgufContextError {
    GgufContextError::KeyNotFound {
        key: "pointer_head.delimiter.decide".to_owned(),
    }
}

#[test]
fn a_pointer_head_without_its_delimiters_is_refused() {
    let load_error = PointerHead::load(&fixture_path("pointer_head_without_delimiters.gguf")).err();

    assert_eq!(
        load_error.as_ref().map(ToString::to_string),
        Some(
            PointerHeadError::KeyUnreadable {
                key: "pointer_head.delimiter.decide".to_owned(),
                source: decide_delimiter_not_found(),
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
        Some(decide_delimiter_not_found().to_string())
    );
}
