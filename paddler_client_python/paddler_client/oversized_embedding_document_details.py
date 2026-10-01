from __future__ import annotations

from dataclasses import dataclass
from typing import Any


@dataclass(frozen=True)
class OversizedEmbeddingDocumentDetails:
    document_tokens: int
    n_batch: int
    source_document_id: str

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> OversizedEmbeddingDocumentDetails:
        return cls(
            document_tokens=int(data["document_tokens"]),
            n_batch=int(data["n_batch"]),
            source_document_id=str(data["source_document_id"]),
        )
