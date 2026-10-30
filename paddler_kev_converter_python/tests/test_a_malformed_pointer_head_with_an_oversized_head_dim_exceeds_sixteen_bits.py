from pathlib import Path

from tests.tensor_shapes import tensor_shapes
from tests.written_malformed_pointer_head import written_malformed_pointer_head


def test_a_malformed_pointer_head_with_an_oversized_head_dim_exceeds_sixteen_bits(
    tmp_path: Path,
) -> None:
    shapes = tensor_shapes(
        written_malformed_pointer_head(tmp_path, "oversized-head-dim")
    )

    assert shapes["pointer_head.key.bias"] == (2**16,)
