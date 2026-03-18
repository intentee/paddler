from pydantic import BaseModel

from paddler_client.types.agent_controller_snapshot import AgentControllerSnapshot


class AgentControllerPoolSnapshot(BaseModel):
    agents: list[AgentControllerSnapshot]
