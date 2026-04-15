from pydantic import BaseModel


class EmbeddingInputDocument(BaseModel):
    content: str
    id: str
