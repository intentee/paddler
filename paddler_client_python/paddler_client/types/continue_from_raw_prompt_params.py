from pydantic import BaseModel


class ContinueFromRawPromptParams(BaseModel):
    max_tokens: int
    raw_prompt: str
