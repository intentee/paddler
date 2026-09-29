from dataclasses import dataclass


@dataclass(frozen=True)
class AgentSpec:
    name: str
    slots: int
