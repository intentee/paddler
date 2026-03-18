from pydantic import BaseModel


class ModelMetadata(BaseModel):
    metadata: dict[str, str] = {}
