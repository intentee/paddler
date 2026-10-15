use std::error::Error;

use llama_cpp_bindings::gguf_context_error::GgufContextError;

use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

fn temperature_not_found() -> GgufContextError {
    GgufContextError::KeyNotFound {
        key: "pointer_head.temperature".to_owned(),
    }
}

#[test]
fn a_pointer_head_without_its_temperature_is_refused() {
    let load_error =
        PointerHead::load(&fixture_path("pointer_head_without_temperature.gguf")).err();

    assert_eq!(
        load_error.as_ref().map(ToString::to_string),
        Some(
            PointerHeadError::KeyUnreadable {
                key: "pointer_head.temperature".to_owned(),
                source: temperature_not_found(),
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
        Some(temperature_not_found().to_string())
    );
}
