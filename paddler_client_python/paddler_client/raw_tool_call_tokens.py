from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class RawToolCallTokens:
    text: str
    ffi_error_message: str
    synthetic_render_with_tools: str
    synthetic_render_without_tools: str

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> RawToolCallTokens:
        return cls(
            text=str(data["text"]),
            ffi_error_message=str(data["ffi_error_message"]),
            synthetic_render_with_tools=str(data["synthetic_render_with_tools"]),
            synthetic_render_without_tools=str(data["synthetic_render_without_tools"]),
        )
