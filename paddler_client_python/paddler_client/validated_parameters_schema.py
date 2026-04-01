from typing import Any

from pydantic import BaseModel, ConfigDict, Field


class ValidatedParametersSchema(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    schema_type: str = Field(alias="type")
    properties: dict[str, Any] | None = None
    required: list[str] | None = None
    additional_properties: Any | None = Field(
        default=None,
        alias="additionalProperties",
    )
