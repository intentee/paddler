import pytest

from paddler_client.parsed_tool_call import ParsedToolCall
from paddler_client.tool_call_arguments import InvalidJson
from paddler_client.tool_call_arguments import ValidJson


def test_from_dict_with_valid_json_arguments() -> None:
    parsed = ParsedToolCall.from_dict(
        {
            "id": "call_42",
            "name": "get_weather",
            "arguments": {"ValidJson": {"location": "Paris"}},
        },
    )

    assert parsed.id == "call_42"
    assert parsed.name == "get_weather"
    assert parsed.arguments == ValidJson({"location": "Paris"})


def test_from_dict_with_invalid_json_arguments() -> None:
    parsed = ParsedToolCall.from_dict(
        {
            "id": "call_99",
            "name": "freeform",
            "arguments": {"InvalidJson": "{half a json"},
        },
    )

    assert parsed.arguments == InvalidJson("{half a json")


def test_from_dict_with_non_dict_arguments_raises() -> None:
    with pytest.raises(ValueError, match="arguments field must be a dict"):
        ParsedToolCall.from_dict(
            {
                "id": "x",
                "name": "y",
                "arguments": "not a dict",
            },
        )
