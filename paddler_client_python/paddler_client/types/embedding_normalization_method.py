from __future__ import annotations

from typing import Any

from pydantic import BaseModel, ConfigDict, model_serializer, model_validator


class EmbeddingNormalizationMethod(BaseModel):
    model_config = ConfigDict(frozen=True)

    variant: str
    epsilon: float | None = None

    @model_validator(mode="before")
    @classmethod
    def from_serde(cls, data: Any) -> dict[str, Any]:
        if isinstance(data, str):
            return {"variant": data}

        if isinstance(data, dict):
            if "RmsNorm" in data:
                inner = data["RmsNorm"]

                if not isinstance(inner, dict) or "epsilon" not in inner:
                    raise ValueError(f"Invalid RmsNorm payload: {data}")

                return {
                    "variant": "RmsNorm",
                    "epsilon": inner["epsilon"],
                }

            if "variant" in data:
                return data

        raise ValueError(f"Invalid EmbeddingNormalizationMethod: {data}")

    @model_serializer
    def to_serde(self) -> str | dict[str, Any]:
        if self.variant == "RmsNorm":
            if self.epsilon is None:
                raise ValueError("epsilon is required for RmsNorm")

            return {"RmsNorm": {"epsilon": self.epsilon}}

        return self.variant

    @classmethod
    def l2(cls) -> EmbeddingNormalizationMethod:
        return cls(variant="L2")

    @classmethod
    def none(cls) -> EmbeddingNormalizationMethod:
        return cls(variant="None")

    @classmethod
    def rms_norm(cls, epsilon: float) -> EmbeddingNormalizationMethod:
        return cls(variant="RmsNorm", epsilon=epsilon)
