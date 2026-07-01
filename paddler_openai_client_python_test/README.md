# paddler_openai_client_python_test

Verifies that the **official OpenAI Python client** works against Paddler's OpenAI-compatible
endpoints (`/v1/chat/completions` and `/v1/responses`). It depends on the official `openai` package
only — never on Paddler's own client — and exercises nothing but the OpenAI endpoints, so a passing
run is objective evidence that a real OpenAI client is compatible with the server.

It does not start or configure a cluster. Point it at an already-running, model-configured Paddler
server via `PADDLER_OPENAI_BASE_URL`; the suite fails if that variable is not set.

## Running

```sh
poetry install
PADDLER_OPENAI_BASE_URL=http://127.0.0.1:8063/v1 poetry run pytest
```

- `PADDLER_OPENAI_BASE_URL` (required): base URL of the running endpoint, ending in `/v1`.
- `PADDLER_OPENAI_MODEL` (optional, default `qwen3`): the model name to send. Paddler ignores it,
  but the OpenAI client requires one.
