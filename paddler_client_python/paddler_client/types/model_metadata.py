from pydantic import BaseModel, Field


class ModelMetadata(BaseModel):
    metadata: dict[str, str] = Field(default_factory=dict)
