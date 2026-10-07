# paddler_typesafe_client_python_test

Verifies that the official **TypeSafe Python SDK** (`typesafe-sdk`) works against Paddler's
TypeSafe-compatible endpoints (`POST /v1/systemone` and `GET /v1/models`). The tests drive those
endpoints with the SDK only — never with Paddler's own client — so a passing run is objective
evidence that a real TypeSafe client only needs to change its base URL.

The suite runs inside `run_with_test_cluster` from `paddler_tests`, which starts a Paddler cluster
in decision mode serving Kev-0.8B, converted locally into `target/kev` by the repository's
Makefile, and passes the TypeSafe compatibility service's address to the tests in
`PADDLER_COMPAT_TYPESAFE_URL`. Paddler has no authentication, so the SDK's required API key is
sent and ignored.

## Running

From the repository root:

```sh
TEST_DEVICE=cuda make test.typesafe.python.llm
```
