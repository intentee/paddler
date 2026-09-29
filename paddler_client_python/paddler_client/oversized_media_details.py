from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class OversizedMediaDetails:
    media_tokens: int
    micro_batch_tokens: int

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> OversizedMediaDetails:
        return cls(
            media_tokens=int(data["media_tokens"]),
            micro_batch_tokens=int(data["micro_batch_tokens"]),
        )
