from pathlib import Path

from tests.written_malformed_pointer_head import written_malformed_pointer_head


def test_a_malformed_pointer_head_without_its_delimiters_keeps_its_temperature(
    tmp_path: Path,
) -> None:
    reader = written_malformed_pointer_head(tmp_path, "missing-delimiters")

    assert [
        field_name
        for field_name in reader.fields
        if field_name.startswith("pointer_head.delimiter.")
    ] == []
    assert "pointer_head.temperature" in reader.fields
