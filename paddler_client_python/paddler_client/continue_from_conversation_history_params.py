from pydantic import BaseModel, PositiveInt, model_validator

from paddler_client.conversation_message import ConversationMessage
from paddler_client.error import ToolCallParsingWithoutToolsError
from paddler_client.grammar_constraint import GrammarConstraint
from paddler_client.tool import Tool


class ContinueFromConversationHistoryParams(BaseModel):
    add_generation_prompt: bool
    conversation_history: list[ConversationMessage]
    enable_thinking: bool
    grammar: GrammarConstraint | None = None
    max_tokens: PositiveInt
    parse_tool_calls: bool = False
    tools: list[Tool] = []

    @model_validator(mode="after")
    def reject_tool_call_parsing_without_tools(
        self,
    ) -> "ContinueFromConversationHistoryParams":
        if self.parse_tool_calls and not self.tools:
            raise ToolCallParsingWithoutToolsError

        return self
