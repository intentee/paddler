from pydantic import BaseModel

from paddler_client.types.agent_desired_model import AgentDesiredModel
from paddler_client.types.chat_template import ChatTemplate
from paddler_client.types.inference_parameters import InferenceParameters


class BalancerDesiredState(BaseModel):
    chat_template_override: ChatTemplate | None = None
    inference_parameters: InferenceParameters = InferenceParameters()
    model: AgentDesiredModel = AgentDesiredModel.none()
    multimodal_projection: AgentDesiredModel = AgentDesiredModel.none()
    use_chat_template_override: bool = False
