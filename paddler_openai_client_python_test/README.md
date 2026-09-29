# paddler_openai_client_python_test

Verifies that the **OpenAI Python client** works against Paddler's OpenAI-compatible
endpoints (`/v1/chat/completions` and `/v1/responses`). The tests drive those endpoints with the
`openai` package only — never with Paddler's own client — so a passing run is objective evidence
that a real OpenAI client is compatible with the server.

Each test spawns its own Paddler balancer and a `Qwen3-0.6B` agent through `paddler_test_cluster`,
using the binary that `PADDLER_BINARY` points at.

## Running

From the repository root:

```sh
TEST_DEVICE=cuda make test.openai.python.llm
```
