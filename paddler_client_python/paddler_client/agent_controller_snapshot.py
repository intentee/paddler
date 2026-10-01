from pydantic import BaseModel

from paddler_client.agent_status import AgentStatus


class AgentControllerSnapshot(BaseModel):
    id: str
    name: str | None = None
    slots_processing: int
    status: AgentStatus
