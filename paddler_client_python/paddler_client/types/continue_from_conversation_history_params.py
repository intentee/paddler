from pydantic import BaseModel, Field

from paddler_client.types.conversation_message import ConversationMessage
from paddler_client.types.tool import Tool


class ContinueFromConversationHistoryParams(BaseModel):
    add_generation_prompt: bool
    conversation_history: list[ConversationMessage]
    enable_thinking: bool
    max_tokens: int
    tools: list[Tool] = Field(default_factory=list)
