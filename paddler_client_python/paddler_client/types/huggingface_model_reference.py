from pydantic import BaseModel


class HuggingFaceModelReference(BaseModel):
    filename: str
    repo_id: str
    revision: str
