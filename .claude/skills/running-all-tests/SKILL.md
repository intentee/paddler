---
name: running-all-tests
description: Runs every test suite in the paddler workspace on the fastest available device. Use when the user asks to run the tests, run all the tests, run the full test suite, or check that everything still passes.
---

# Running all tests

Run every test suite in the workspace, picking the fastest compiled device backend for the host. 

## Step 1: detect the device

Run this once at the start and echo the chosen device:

```bash
if [[ "$OSTYPE" == "darwin"* ]]; then
  DEVICE=metal
elif command -v nvidia-smi >/dev/null 2>&1 && nvidia-smi >/dev/null 2>&1; then
  DEVICE=cuda
else
  DEVICE=cpu
fi
echo "Device: $DEVICE"
```

`$DEVICE` selects the Paddler binary and the Rust feature set every suite in Step 2 runs against.

## Step 2: run the suites

Copy this checklist and tick each item as the suite completes:

```
- [ ] JS client
- [ ] JS client LLM
- [ ] Python lint
- [ ] OpenAI Python client LLM
- [ ] TypeSafe Python client LLM
- [ ] kev converter LLM
- [ ] Rust unit
- [ ] Rust integration
```

| # | Suite                      | Command (from the repo root)                                          |
|---|----------------------------|-----------------------------------------------------------------------|
| 1 | JS client                  | `TEST_DEVICE=$DEVICE make test.client.js`                             |
| 2 | JS client LLM              | `TEST_DEVICE=$DEVICE make test.client.js.llm`                         |
| 3 | Python lint                | `make lint.openai.python lint.typesafe.python lint.kev_converter.python` |
| 4 | OpenAI Python client LLM   | `TEST_DEVICE=$DEVICE make test.openai.python.llm`                     |
| 5 | TypeSafe Python client LLM | `TEST_DEVICE=$DEVICE make test.typesafe.python.llm`                   |
| 6 | kev converter LLM          | `make test.kev_converter.python.llm`                                  |
| 7 | Rust unit                  | `TEST_DEVICE=$DEVICE make test.unit`                                  |
| 8 | Rust integration           | `TEST_DEVICE=$DEVICE make test.integration`                           |

Run them in this order. Cheap suites (1, 3, 7) surface bugs quickly; the suites that load models (2, 4, 5, 6, 8) come after them.

On NixOS the pinned `ruff` wheel is dynamically linked, so suite 3 needs `nix-ld`.

## Step 3: rules during the run

- **Serialize GPU suites.** When `$DEVICE` is `cuda` or `metal`, run test suites sequentially to avoid device contention. The repository enforces one GPU test at a time: the Makefile is `.NOTPARALLEL`, nextest's default profile runs a single test (only `TEST_DEVICE=cpu` selects the parallel `cpu` profile), the JS suites run with `--test-concurrency=1`, and `.cargo/config.toml` sets `RUST_TEST_THREADS=1`. Never override that with `NEXTEST_PROFILE=cpu`, `--test-threads`, or `make -j` on a GPU build.
- **Per-test 30 s budget.** Flag any individual test that exceeds 30 s wall-clock. That is a real bug — production or test — not flakiness.

## Step 4: report

After all suites finish, sum up the results in an actionable report.
