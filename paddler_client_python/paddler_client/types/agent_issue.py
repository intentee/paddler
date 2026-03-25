from __future__ import annotations

from typing import Any

from pydantic import BaseModel, Field, model_serializer, model_validator


class AgentIssue(BaseModel):
    variant: str
    params: dict[str, Any] = Field(default_factory=dict)

    @model_validator(mode="before")
    @classmethod
    def from_serde(cls, data: Any) -> dict[str, Any]:
        if isinstance(data, str):
            return {"variant": data, "params": {}}

        if isinstance(data, dict):
            if "variant" in data:
                return data

            if len(data) == 1:
                variant, params = next(iter(data.items()))

                if isinstance(params, dict):
                    return {"variant": variant, "params": params}

                return {"variant": variant, "params": {"value": params}}

        raise ValueError(f"Invalid AgentIssue: {data}")

    @model_serializer
    def to_serde(self) -> str | dict[str, Any]:
        if not self.params:
            return self.variant

        return {self.variant: self.params}
