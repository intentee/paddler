from pathlib import Path

from tests.tensor_shapes import tensor_shapes
from tests.written_malformed_pointer_head import written_malformed_pointer_head


def test_a_malformed_pointer_head_with_mismatched_projections_shapes_query_unlike_key(
    tmp_path: Path,
) -> None:
    shapes = tensor_shapes(
        written_malformed_pointer_head(tmp_path, "mismatched-projections")
    )

    assert shapes["pointer_head.query.weight"] != shapes["pointer_head.key.weight"]
