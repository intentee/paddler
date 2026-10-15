use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

#[test]
fn a_pointer_head_with_zero_temperature_is_refused() {
    assert_eq!(
        PointerHead::load(&fixture_path("pointer_head_with_zero_temperature.gguf"))
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(PointerHeadError::TemperatureOutOfRange { temperature: 0.0 }.to_string())
    );
}
