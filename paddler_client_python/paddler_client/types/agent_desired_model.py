from typing import Any, cast

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
            typed_data = cast("dict[str, Any]", data)

            if "HuggingFace" in typed_data:
                return {
                    "variant": "HuggingFace",
                    "huggingface": typed_data["HuggingFace"],
                }

            if "LocalToAgent" in typed_data:
                return {
                    "variant": "LocalToAgent",
                    "local_path": typed_data["LocalToAgent"],
                }

            if "variant" in typed_data:
                return typed_data

        msg = f"Invalid AgentDesiredModel: {data}"
        raise ValueError(msg)

    @model_serializer
    def to_serde(self) -> str | dict[str, Any]:
        if self.variant == "None":
            return "None"

        if self.variant == "HuggingFace" and self.huggingface is not None:
            return {"HuggingFace": self.huggingface.model_dump()}

        if self.variant == "LocalToAgent":
            if self.local_path is None:
                msg = "local_path is required for LocalToAgent"
                raise ValueError(msg)

            return {"LocalToAgent": self.local_path}

        msg = f"Unknown AgentDesiredModel variant: {self.variant}"
        raise ValueError(msg)

    @classmethod
    def none(cls) -> "AgentDesiredModel":
        return cls(variant="None")

    @classmethod
    def from_huggingface(
        cls, reference: HuggingFaceModelReference
    ) -> "AgentDesiredModel":
        return cls(variant="HuggingFace", huggingface=reference)

    @classmethod
    def local_to_agent(cls, path: str) -> "AgentDesiredModel":
        return cls(variant="LocalToAgent", local_path=path)
