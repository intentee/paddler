from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class OversizedPromptDetails:
    prompt_tokens: int
    sequence_context_size: int

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> OversizedPromptDetails:
        return cls(
            prompt_tokens=int(data["prompt_tokens"]),
            sequence_context_size=int(data["sequence_context_size"]),
        )
