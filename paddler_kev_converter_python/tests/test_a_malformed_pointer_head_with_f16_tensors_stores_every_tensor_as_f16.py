from pathlib import Path

from gguf import GGMLQuantizationType

from tests.written_malformed_pointer_head import written_malformed_pointer_head


def test_a_malformed_pointer_head_with_f16_tensors_stores_every_tensor_as_f16(
    tmp_path: Path,
) -> None:
    assert {
        tensor.tensor_type
        for tensor in written_malformed_pointer_head(tmp_path, "f16-tensors").tensors
    } == {GGMLQuantizationType.F16}
