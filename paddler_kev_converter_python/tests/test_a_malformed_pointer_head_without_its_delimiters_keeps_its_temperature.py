from pathlib import Path

from tests.written_malformed_pointer_head import written_malformed_pointer_head


def test_a_malformed_pointer_head_without_its_delimiters_keeps_its_temperature(
    tmp_path: Path,
) -> None:
    reader = written_malformed_pointer_head(tmp_path, "missing-delimiters")

    assert "pointer_head.delimiter.state" not in reader.fields
    assert "pointer_head.temperature" in reader.fields
