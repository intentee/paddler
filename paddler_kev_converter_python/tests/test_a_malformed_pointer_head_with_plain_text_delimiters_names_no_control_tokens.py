from pathlib import Path

from tests.written_malformed_pointer_head import written_malformed_pointer_head


def test_a_malformed_pointer_head_with_plain_text_delimiters_names_no_control_tokens(
    tmp_path: Path,
) -> None:
    reader = written_malformed_pointer_head(tmp_path, "plain-text-delimiters")

    assert reader.fields["pointer_head.delimiter.state"].contents() == "state"
