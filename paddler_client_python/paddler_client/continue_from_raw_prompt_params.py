from pydantic import BaseModel, PositiveInt

from paddler_client.grammar_constraint import GrammarConstraint


class ContinueFromRawPromptParams(BaseModel):
    grammar: GrammarConstraint | None = None
    max_tokens: PositiveInt
    raw_prompt: str
