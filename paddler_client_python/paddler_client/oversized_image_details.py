from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class OversizedImageDetails:
    image_tokens: int
    n_batch: int

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> OversizedImageDetails:
        return cls(
            image_tokens=int(data["image_tokens"]),
            n_batch=int(data["n_batch"]),
        )
