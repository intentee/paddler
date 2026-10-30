from pathlib import Path

from tests.written_malformed_pointer_head import written_malformed_pointer_head


def test_a_malformed_pointer_head_with_zero_temperature_stores_a_zero_temperature(
    tmp_path: Path,
) -> None:
    reader = written_malformed_pointer_head(tmp_path, "zero-temperature")

    assert reader.fields["pointer_head.temperature"].contents() == 0.0
