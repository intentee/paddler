from enum import StrEnum


class GenerationFinish(StrEnum):
    END_OF_GENERATION = "EndOfGeneration"
    MAX_TOKENS = "MaxTokens"
    STOP_REQUESTED = "StopRequested"
