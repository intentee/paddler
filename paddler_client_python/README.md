# paddler-client

Python client for the [Paddler](https://github.com/intentee/paddler) LLM load balancer.

## Install

```sh
pip install paddler-client
```

Requires Python 3.11+. Built on `httpx`, `websockets`, and `pydantic` v2.

## Quick start

### WebSocket inference (multiplexed, request-id-keyed)

```python
import asyncio

from paddler_client import ClientInference
from paddler_client.continue_from_conversation_history_params import (
    ContinueFromConversationHistoryParams,
)
from paddler_client.conversation_message import ConversationMessage


async def main() -> None:
    params = ContinueFromConversationHistoryParams(
        add_generation_prompt=True,
        conversation_history=[
            ConversationMessage(content="Say hello.", role="user"),
        ],
        enable_thinking=False,
        max_tokens=128,
    )

    async with ClientInference(url="http://localhost:8061") as client:
        stream = await client.continue_from_conversation_history(params)

        async for message in stream:
            if message.is_token:
                print(message.token, end="", flush=True)
            elif message.is_terminal:
                break


asyncio.run(main())
```

### HTTP NDJSON streaming

```python
async with ClientInference(url="http://localhost:8061") as client:
    async for message in client.post_continue_from_conversation_history(params):
        if message.is_token:
            print(message.token, end="", flush=True)
        elif message.is_terminal:
            break
```

`ClientInference` also provides `continue_from_raw_prompt` and
`generate_embedding_batch`.

### Management

```python
from paddler_client import ClientManagement


async with ClientManagement(url="http://localhost:8062") as client:
    snapshot = await client.get_agents()

    for agent in snapshot.agents:
        print(agent.id, agent.slots_total)

    state = await client.get_balancer_desired_state()
    await client.put_balancer_desired_state(state)

    metrics = await client.get_metrics()
```

`ClientManagement` also provides `agents_stream`, `get_buffered_requests`,
`buffered_requests_stream`, `get_chat_template_override`, and
`get_model_metadata`.

## Coverage

- Transport: WebSocket (multiplexed), HTTP NDJSON, HTTP JSON, Server-Sent Events
- Models: every Paddler wire-format type (validated via pydantic)
- Fully typed (`py.typed`), async-first API
- Specialized error types per failure mode

## License

Apache-2.0
