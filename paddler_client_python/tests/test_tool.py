from paddler_client.tool import Function, Tool
from paddler_client.validated_parameters_schema import (
    ValidatedParametersSchema,
)


def test_tool_serialization() -> None:
    tool = Tool(
        function=Function(
            name="get_weather",
            description="Get weather",
        )
    )
    dumped = tool.model_dump(mode="json", exclude_none=True)

    assert dumped == {
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get weather",
        },
    }


def test_tool_with_parameters_serialization() -> None:
    tool = Tool(
        function=Function(
            name="get_weather",
            description="Get weather",
            parameters=ValidatedParametersSchema.model_validate({
                "type": "object",
                "properties": {"location": {"type": "string"}},
                "required": ["location"],
            }),
        )
    )
    dumped = tool.model_dump(
        mode="json",
        exclude_none=True,
        by_alias=True,
    )

    assert dumped["function"]["parameters"]["type"] == "object"
    assert "location" in dumped["function"]["parameters"]["properties"]
    assert dumped["function"]["parameters"]["required"] == ["location"]


def test_tool_deserialization() -> None:
    tool = Tool.model_validate(
        {
            "type": "function",
            "function": {"name": "test", "description": "desc"},
        }
    )

    assert tool.function.name == "test"
