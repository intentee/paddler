# paddler_openai_client_python_test

Verifies that the official **OpenAI Python SDK** (`openai`) works against Paddler's
OpenAI-compatible endpoints (`POST /v1/chat/completions` and `POST /v1/responses`). The tests
drive those endpoints with the SDK only — never with Paddler's own client — so a passing run is
objective evidence that a real OpenAI client only needs to change its base URL.

The suite runs inside `run_with_test_cluster` from `paddler_tests`, which starts a Paddler cluster
in text generation mode serving Qwen3-0.6B and passes the OpenAI compatibility service's address
to the tests in `PADDLER_COMPAT_OPENAI_URL`. Paddler has no authentication, so the SDK's required
API key is sent and ignored.

## Running

From the repository root:

```sh
TEST_DEVICE=cuda make test.openai.python.llm
```
