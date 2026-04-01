from typing import Any, cast

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
            typed_data = cast("dict[str, Any]", data)

            if "variant" in typed_data:
                return typed_data

            if len(typed_data) == 1:
                variant, params = next(iter(typed_data.items()))

                if isinstance(params, dict):
                    return {"variant": variant, "params": params}

                return {"variant": variant, "params": {"value": params}}

        msg = f"Invalid AgentIssue: {data}"
        raise ValueError(msg)

    @model_serializer
    def to_serde(self) -> str | dict[str, Any]:
        if not self.params:
            return self.variant

        return {self.variant: self.params}
