from typing import Any

from pydantic import BaseModel

class SystemOneRequest(BaseModel): ...

def to_record(
    req: SystemOneRequest,
) -> tuple[dict[str, Any], list[dict[str, Any]]]: ...
def to_answers(
    probs: list[list[float]], meta: list[dict[str, Any]]
) -> dict[str, Any]: ...
