from pydantic import BaseModel

from paddler_client.conversation_message import ConversationMessage
from paddler_client.grammar_constraint import GrammarConstraint
from paddler_client.tool import Tool


class ContinueFromConversationHistoryParams(BaseModel):
    add_generation_prompt: bool
    conversation_history: list[ConversationMessage]
    enable_thinking: bool
    grammar: GrammarConstraint | None = None
    max_tokens: int
    tools: list[Tool] = []
