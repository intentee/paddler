---
paths:
  - "paddler_balancer/**"
---

# Paddler Balancer Context

- `paddler_balancer` crate is responsible for starting inference, and management servers
- Paddler Agents connect to the balancer in order to handle the requests that the balancer dispatches
- `paddler_balancer` provides compatibility services that expose vendor-compatible APIs: OpenAI compatibility (built on `paddler_openai_translation`), and TypeSafe System One compatibility (built on `paddler_typesafe_translation`); each one is served whenever its address is configured

# Inference Modes

- the balancer desired state is the only source of truth for the inference mode; reconciliation transforms it into the applicable state, and agents receive only the active mode's settings
- every route is registered in every mode; each handler first checks the applied mode with `BalancerApplicableStateHolder::require_inference_mode`, or reads the applied settings once and matches the variant it needs
- a request of another mode is refused with `ClusterServesAnotherInferenceMode`: 503 over HTTP and in every compatibility layer, an in-band `InferenceModeMismatch` over the inference socket
- the agent pool dispatches by free capacity only; an agent still serving the previous mode answers `InferenceModeMismatch`
- the inference socket notifies its clients of the cluster's inference mode on connect and whenever it changes

# Compatibility Layers

Every compatibility layer follows the same conventions, so adding a vendor never means inventing a new shape:

- the HTTP layer lives in `compatibility/<vendor>_service/`; vendor DTOs, request translation, response building, and generation state live in a pure `paddler_<vendor>_translation` crate with no actix or balancer dependency and a single `<Vendor>TranslationError`
- each layer is a unit struct `<Vendor>CompatibilityLayer` implementing `ServesCompatibilityLayer`; it runs inside the shared `CompatibilityService<TCompatibilityLayer>`, configured by `CompatibilityServiceConfiguration`, which owns CORS, the request id middleware, `/health`, and the worker count
- paths come from `<Vendor>ApiPath` and headers from `<Vendor>Header`, which always declares `REQUEST_ID`; tests and the harness import these constants instead of repeating string literals
- `attach_request_id` echoes the client's `<Vendor>Header::REQUEST_ID` or generates one, and CORS exposes it
- the wire error body is `<Vendor>ErrorBody` with `into_http_response(self, StatusCode)`; malformed JSON is refused through `<vendor>_json_config()` with the vendor's client-error status (OpenAI 400, TypeSafe 422)
- routes relay agent results through `CompatibilityAppData::agent_result_stream`, which yields `AgentResultStreamEvent<TResult>`; a response of another inference mode is refused as `AgentRelayError::MessageNotRelayable`
- buffered responders are named `<scope>_http_response`, streaming responders `<scope>_sse_response`
- server-side statuses come only from `UpstreamFailure` (500 agent failed, 502 relay failed, 503 unavailable, 504 timed out, and the JSON-RPC code mapping); client-error statuses stay vendor-specific
- a route is `async fn respond(..) -> HttpResponse`, registered by `pub fn <method>_<endpoint>(cfg)`
- integration tests are named `<vendor>_<endpoint|service>_<behavior>`
