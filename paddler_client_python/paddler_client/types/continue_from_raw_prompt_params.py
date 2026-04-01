from pydantic import BaseModel

from paddler_client.types.grammar_constraint import GrammarConstraint


class ContinueFromRawPromptParams(BaseModel):
    grammar: GrammarConstraint | None = None
    max_tokens: int
    raw_prompt: str
