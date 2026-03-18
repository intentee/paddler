from pydantic import BaseModel

from paddler_client.types.agent_issue import AgentIssue
from paddler_client.types.agent_state_application_status import (
    AgentStateApplicationStatus,
)


class AgentControllerSnapshot(BaseModel):
    desired_slots_total: int
    download_current: int
    download_filename: str | None = None
    download_total: int
    id: str
    issues: list[AgentIssue] = []
    model_path: str | None = None
    name: str | None = None
    slots_processing: int
    slots_total: int
    state_application_status: AgentStateApplicationStatus
    uses_chat_template_override: bool
