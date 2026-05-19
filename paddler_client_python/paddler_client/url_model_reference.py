from pydantic import BaseModel


class UrlModelReference(BaseModel):
    url: str
