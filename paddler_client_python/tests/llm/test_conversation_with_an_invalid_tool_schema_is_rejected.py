from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_inference import ClientInference
from paddler_client.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.conversation_message import ConversationMessage
from paddler_client.inference_message import InferenceMessageKind
from paddler_client.tool import Function, Tool
from paddler_client.validated_parameters_schema import ValidatedParametersSchema


async def test_conversation_with_an_invalid_tool_schema_is_rejected(
    qwen3_cluster: BalancerAddresses,
) -> None:
    async with ClientInference(url=qwen3_cluster.inference_url) as client:
        kinds = [
            message.kind
            async for message in client.post_continue_from_conversation_history(
                ContinueFromConversationHistoryParams(
                    add_generation_prompt=True,
                    conversation_history=[
                        ConversationMessage(
                            content="What is the weather in Paris?", role="user"
                        )
                    ],
                    enable_thinking=False,
                    max_tokens=64,
                    parse_tool_calls=True,
                    tools=[
                        Tool(
                            function=Function(
                                name="get_weather",
                                description="Get the current weather for a location",
                                parameters=ValidatedParametersSchema(
                                    schema_type="object",
                                    properties={"location": {"type": 123}},
                                ),
                            )
                        )
                    ],
                )
            )
        ]

    assert kinds == [InferenceMessageKind.TOOL_SCHEMA_INVALID]
