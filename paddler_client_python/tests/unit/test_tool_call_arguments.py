import pytest

from paddler_client.tool_call_arguments import (
    InvalidJson,
    ValidJson,
    parse_tool_call_arguments,
)


def test_parse_valid_json_with_object() -> None:
    result = parse_tool_call_arguments({"ValidJson": {"location": "Paris"}})

    assert result == ValidJson({"location": "Paris"})


def test_parse_valid_json_with_array() -> None:
    result = parse_tool_call_arguments({"ValidJson": [1, 2, 3]})

    assert result == ValidJson([1, 2, 3])


def test_parse_invalid_json_carries_raw_text() -> None:
    result = parse_tool_call_arguments({"InvalidJson": "{half a json"})

    assert result == InvalidJson("{half a json")


def test_parse_unknown_shape_raises() -> None:
    with pytest.raises(ValueError, match="Unknown ToolCallArguments shape"):
        parse_tool_call_arguments({"SomethingElse": "x"})
