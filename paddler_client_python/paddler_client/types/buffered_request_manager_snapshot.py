from pydantic import BaseModel


class BufferedRequestManagerSnapshot(BaseModel):
    buffered_requests_current: int
