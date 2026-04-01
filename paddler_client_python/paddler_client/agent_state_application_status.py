from enum import StrEnum


class AgentStateApplicationStatus(StrEnum):
    APPLIED = "Applied"
    ATTEMPTED_AND_NOT_APPLIABLE = "AttemptedAndNotAppliable"
    ATTEMPTED_AND_RETRYING = "AttemptedAndRetrying"
    FRESH = "Fresh"
    STUCK = "Stuck"
