use paddler_agent_pointer_head::pointer_head::PointerHead;
use paddler_agent_pointer_head::pointer_head_error::PointerHeadError;

use crate::fixture_path::fixture_path;

const OVERSIZED_HEAD_DIM: i64 = 1 << 16;

#[test]
fn a_pointer_head_whose_head_dimension_exceeds_sixteen_bits_is_refused() {
    assert_eq!(
        PointerHead::load(&fixture_path("pointer_head_with_oversized_head_dim.gguf"))
            .err()
            .as_ref()
            .map(ToString::to_string),
        Some(
            PointerHeadError::HeadDimensionOutOfRange {
                head_dim: OVERSIZED_HEAD_DIM,
                source: u16::try_from(OVERSIZED_HEAD_DIM)
                    .expect_err("the head dimension must not fit sixteen bits"),
            }
            .to_string()
        )
    );
}
