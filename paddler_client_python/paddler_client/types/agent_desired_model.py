from __future__ import annotations

from typing import Any

from pydantic import BaseModel, ConfigDict, model_serializer, model_validator

from paddler_client.types.huggingface_model_reference import (
    HuggingFaceModelReference,
)


class AgentDesiredModel(BaseModel):
    model_config = ConfigDict(frozen=True)

    variant: str
    huggingface: HuggingFaceModelReference | None = None
    local_path: str | None = None

    @model_validator(mode="before")
    @classmethod
    def from_serde(cls, data: Any) -> dict[str, Any]:
        if isinstance(data, str) and data == "None":
            return {"variant": "None"}

        if isinstance(data, dict):
            if "HuggingFace" in data:
                return {
                    "variant": "HuggingFace",
                    "huggingface": data["HuggingFace"],
                }

            if "LocalToAgent" in data:
                return {
                    "variant": "LocalToAgent",
                    "local_path": data["LocalToAgent"],
                }

            if "variant" in data:
                return data

        raise ValueError(f"Invalid AgentDesiredModel: {data}")

    @model_serializer
    def to_serde(self) -> str | dict[str, Any]:
        if self.variant == "None":
            return "None"

        if self.variant == "HuggingFace" and self.huggingface is not None:
            return {"HuggingFace": self.huggingface.model_dump()}

        if self.variant == "LocalToAgent":
            return {"LocalToAgent": self.local_path}

        raise ValueError(f"Unknown AgentDesiredModel variant: {self.variant}")

    @classmethod
    def none(cls) -> AgentDesiredModel:
        return cls(variant="None")

    @classmethod
    def from_huggingface(
        cls, reference: HuggingFaceModelReference
    ) -> AgentDesiredModel:
        return cls(variant="HuggingFace", huggingface=reference)

    @classmethod
    def local_to_agent(cls, path: str) -> AgentDesiredModel:
        return cls(variant="LocalToAgent", local_path=path)
