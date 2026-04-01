from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field


class GbnfGrammarConstraint(BaseModel):
    type: Literal["gbnf"] = "gbnf"
    grammar: str
    root: str


class JsonSchemaGrammarConstraint(BaseModel):
    model_config = ConfigDict(populate_by_name=True)

    type: Literal["json_schema"] = "json_schema"
    schema_value: Annotated[str, Field(alias="schema")]


GrammarConstraint = GbnfGrammarConstraint | JsonSchemaGrammarConstraint
