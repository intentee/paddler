from pathlib import Path

from tests.written_malformed_pointer_head import written_malformed_pointer_head


def test_a_malformed_pointer_head_without_its_temperature_keeps_its_delimiters(
    tmp_path: Path,
) -> None:
    reader = written_malformed_pointer_head(tmp_path, "missing-temperature")

    assert "pointer_head.temperature" not in reader.fields
    assert "pointer_head.delimiter.state" in reader.fields
