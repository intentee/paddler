from pydantic import BaseModel

from paddler_client.conversation_message_content_part import (
    ConversationMessageContentPart,
)

ConversationMessageContent = str | list[ConversationMessageContentPart]


class ConversationMessage(BaseModel):
    content: ConversationMessageContent
    role: str
