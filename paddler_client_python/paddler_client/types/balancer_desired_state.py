from pydantic import BaseModel, Field

from paddler_client.types.agent_desired_model import AgentDesiredModel
from paddler_client.types.chat_template import ChatTemplate
from paddler_client.types.inference_parameters import InferenceParameters


class BalancerDesiredState(BaseModel):
    chat_template_override: ChatTemplate | None = None
    inference_parameters: InferenceParameters = Field(
        default_factory=InferenceParameters,
    )
    model: AgentDesiredModel = Field(default_factory=AgentDesiredModel.none)
    multimodal_projection: AgentDesiredModel = Field(
        default_factory=AgentDesiredModel.none,
    )
    use_chat_template_override: bool = False
