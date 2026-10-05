from __future__ import annotations

from typing import Any


def unparsed_message_request_id(data: Any) -> str | None:
    if not isinstance(data, dict):
        return None

    for envelope_name in ("Response", "Error"):
        envelope: Any = data.get(envelope_name)

        if isinstance(envelope, dict) and "request_id" in envelope:
            return str(envelope["request_id"])

    return None
