# Paddler — Stateful Load Balancer for llama.cpp

## Overview

Load balancer + reverse proxy for llama.cpp servers. Distributes requests by available slots, buffers when all busy.

## Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI (balancer / agent subcommands) |
| `src/balancer/proxy_service.rs` | Proxy logic, model detection (`--check-model`), upstream peer creation |
| `src/balancer/upstream_peer_pool.rs` | Peer registry, `use_best_peer()`, slot tracking |
| `src/balancer/upstream_peer.rs` | Peer struct, `Ord` sort (usable → most idle → least processing) |
| `src/balancer/request_context.rs` | Per-request context, `take_slot()` / `release_slot()` |
| `src/agent/monitoring_service.rs` | Periodic llama.cpp `/slots` polling |
| `src/agent/reporting_service.rs` | Agent → balancer WebSocket keepalive |
| `src/llamacpp/llamacpp_client.rs` | llama.cpp API client |

## Slot Tracking

- Per-request: `take_slot()` / `release_slot()` update counts immediately on request/response
- Heartbeat: agent polls llama.cpp every N ms (`--monitoring-interval`, default 10000), reconciles drift
- Peer sort: usable first, then most idle slots DESC, least processing ASC

## Test Infrastructure

- `tests/integration_tests/` — Rust integration tests (cucumber-style)
- `tests/scripts/test-model-detection.sh` — bash test suite, 19 scenarios
  - `PADDLER_PROXY_HOST=<host> PADDLER_MODEL=<model> bash tests/scripts/test-model-detection.sh`

## Makefile Targets

`build` · `test` · `integration_tests` · `shell_tests` · `fmt` · `clean`
