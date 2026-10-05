from paddler_test_cluster.balancer_addresses import BalancerAddresses

from paddler_client.client_inference import ClientInference
from paddler_client.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.conversation_message import ConversationMessage
from paddler_client.inference_message import InferenceMessageKind
from paddler_client.tool import Function, Tool
from paddler_client.validated_parameters_schema import ValidatedParametersSchema


async def test_tool_call_over_the_inference_socket_streams_until_done(
    qwen3_cluster: BalancerAddresses,
) -> None:
    async with ClientInference(url=qwen3_cluster.inference_url) as client:
        messages = [
            message
            async for message in await client.continue_from_conversation_history(
                ContinueFromConversationHistoryParams(
                    add_generation_prompt=True,
                    conversation_history=[
                        ConversationMessage(
                            content=(
                                "What is the weather in Paris? "
                                "Use the get_weather tool to find out."
                            ),
                            role="user",
                        )
                    ],
                    enable_thinking=False,
                    max_tokens=400,
                    parse_tool_calls=True,
                    tools=[
                        Tool(
                            function=Function(
                                name="get_weather",
                                description="Get the current weather for a location",
                                parameters=ValidatedParametersSchema(
                                    schema_type="object",
                                    properties={
                                        "location": {
                                            "type": "string",
                                            "description": "The city name",
                                        }
                                    },
                                    required=["location"],
                                    additional_properties=False,
                                ),
                            )
                        )
                    ],
                )
            )
        ]

    parsed_tool_call_names = [
        tool_call.name
        for message in messages
        if message.parsed_tool_calls is not None
        for tool_call in message.parsed_tool_calls
    ]

    assert parsed_tool_call_names == ["get_weather"]
    assert messages[-1].kind == InferenceMessageKind.DONE
