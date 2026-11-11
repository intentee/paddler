from pathlib import Path

from tests.written_malformed_pointer_head import written_malformed_pointer_head


def test_a_malformed_pointer_head_with_plain_text_delimiters_names_no_control_tokens(
    tmp_path: Path,
) -> None:
    reader = written_malformed_pointer_head(tmp_path, "plain-text-delimiters")

    assert {
        delimiter_name: reader.fields[f"pointer_head.delimiter.{delimiter_name}"].contents()
        for delimiter_name in ["state", "question", "option_start", "option_end", "decide"]
    } == {
        "state": "state",
        "question": "question",
        "option_start": "option",
        "option_end": "end",
        "decide": "decide",
    }
