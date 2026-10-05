from typing import Any, Self, cast

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
            typed_data = cast("dict[str, Any]", data)

            if "RmsNorm" in typed_data:
                inner: object = typed_data["RmsNorm"]

                if not isinstance(inner, dict) or "epsilon" not in inner:
                    msg = f"Invalid RmsNorm payload: {data}"
                    raise ValueError(msg)

                return {
                    "variant": "RmsNorm",
                    "epsilon": inner["epsilon"],
                }

            if "variant" in typed_data:
                return typed_data

        msg = f"Invalid EmbeddingNormalizationMethod: {data}"
        raise ValueError(msg)

    @model_serializer
    def to_serde(self) -> str | dict[str, Any]:
        if self.variant == "RmsNorm":
            if self.epsilon is None:
                msg = "epsilon is required for RmsNorm"
                raise ValueError(msg)

            return {"RmsNorm": {"epsilon": self.epsilon}}

        return self.variant

    @classmethod
    def l2(cls) -> Self:
        return cls(variant="L2")

    @classmethod
    def none(cls) -> Self:
        return cls(variant="None")

    @classmethod
    def rms_norm(cls, epsilon: float) -> Self:
        return cls(variant="RmsNorm", epsilon=epsilon)
