from pathlib import Path

from tests.tensor_shapes import tensor_shapes
from tests.written_malformed_pointer_head import written_malformed_pointer_head


def test_a_malformed_pointer_head_with_a_zero_head_dim_has_no_projection_rows(
    tmp_path: Path,
) -> None:
    shapes = tensor_shapes(written_malformed_pointer_head(tmp_path, "zero-head-dim"))

    assert shapes["pointer_head.key.bias"] == (0,)
