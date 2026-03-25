from typing import Literal

from pydantic import BaseModel

from paddler_client.types.validated_parameters_schema import (
    ValidatedParametersSchema,
)


class Function(BaseModel):
    name: str
    description: str
    parameters: ValidatedParametersSchema | None = None


class Tool(BaseModel):
    type: Literal["function"] = "function"
    function: Function
