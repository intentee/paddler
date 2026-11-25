# paddler_balancer_openai_service_python_tests

Verifies that the **OpenAI Python client** works against Paddler's OpenAI-compatible
endpoints (`/v1/chat/completions` and `/v1/responses`). The tests drive those endpoints with the
`openai` package only — never with Paddler's own client — so a passing run is objective evidence
that a real OpenAI client is compatible with the server.

The suite runs inside `run_with_test_cluster` from `paddler_cluster_harness`, which starts a Paddler
cluster serving `Qwen3-0.6B` and hands the suite its OpenAI-compatible URL in `PADDLER_COMPAT_OPENAI_URL`.

## Running

From the repository root:

```sh
TEST_DEVICE=cuda make test.openai.python.llm
```
