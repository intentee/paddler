from pydantic import BaseModel

from paddler_client.agent_issue import AgentIssue
from paddler_client.agent_state_application_status import (
    AgentStateApplicationStatus,
)
from paddler_client.model_download_status import ModelDownloadStatus


class AgentStatus(BaseModel):
    desired_slots_total: int
    download_status: ModelDownloadStatus
    issues: list[AgentIssue] = []
    model_path: str | None = None
    slots_total: int
    state_application_status: AgentStateApplicationStatus
    uses_chat_template_override: bool
