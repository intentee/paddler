from __future__ import annotations

from dataclasses import dataclass
from typing import Any
from typing import cast

from paddler_client.tool_call_arguments import ToolCallArguments
from paddler_client.tool_call_arguments import parse_tool_call_arguments


@dataclass(frozen=True)
class ParsedToolCall:
    id: str
    name: str
    arguments: ToolCallArguments

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> ParsedToolCall:
        arguments_payload = data["arguments"]
        if not isinstance(arguments_payload, dict):
            msg = f"arguments field must be a dict (tagged enum), got: {arguments_payload!r}"
            raise ValueError(msg)
        typed_payload = cast("dict[str, Any]", arguments_payload)
        return cls(
            id=str(data["id"]),
            name=str(data["name"]),
            arguments=parse_tool_call_arguments(typed_payload),
        )
