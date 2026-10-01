from enum import StrEnum


class GenerationFinish(StrEnum):
    CONTEXT_FULL = "ContextFull"
    END_OF_GENERATION = "EndOfGeneration"
    MAX_TOKENS = "MaxTokens"
    STOP_REQUESTED = "StopRequested"
