from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from paddler_client.error import UnknownToolCallArgumentsError


@dataclass(frozen=True)
class ValidJson:
    value: Any


@dataclass(frozen=True)
class InvalidJson:
    raw: str


ToolCallArguments = ValidJson | InvalidJson


def parse_tool_call_arguments(payload: dict[str, Any]) -> ToolCallArguments:
    if "ValidJson" in payload:
        return ValidJson(payload["ValidJson"])
    if "InvalidJson" in payload:
        return InvalidJson(str(payload["InvalidJson"]))
    raise UnknownToolCallArgumentsError(payload)
