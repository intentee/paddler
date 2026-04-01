from pydantic import BaseModel


class ChatTemplate(BaseModel):
    content: str
