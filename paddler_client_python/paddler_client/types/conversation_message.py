from pydantic import BaseModel

from paddler_client.types.conversation_message_content_part import (
    ConversationMessageContentPart,
)

ConversationMessageContent = str | list[ConversationMessageContentPart]


class ConversationMessage(BaseModel):
    content: ConversationMessageContent
    role: str
