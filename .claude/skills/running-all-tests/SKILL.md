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
- [ ] Python lint
- [ ] Python client
- [ ] Python client LLM
- [ ] OpenAI Python client LLM
- [ ] Rust unit
- [ ] Rust integration
```

| # | Suite                    | Command (from the repo root)                          |
|---|--------------------------|-------------------------------------------------------|
| 1 | JS client                | `make test.client.js`                                 |
| 2 | Python lint              | `make lint.client.python lint.openai.python`          |
| 3 | Python client            | `TEST_DEVICE=$DEVICE make test.client.python`         |
| 4 | Python client LLM        | `TEST_DEVICE=$DEVICE make test.client.python.llm`     |
| 5 | OpenAI Python client LLM | `TEST_DEVICE=$DEVICE make test.openai.python.llm`     |
| 6 | Rust unit                | `TEST_DEVICE=$DEVICE make test.unit`                  |
| 7 | Rust integration         | `TEST_DEVICE=$DEVICE make test.integration`           |

Run them in this order. Cheap suites (1, 2, 3, 6) surface bugs quickly; the GPU-bound suites (4, 5, 7) load models.

On NixOS the pinned `ruff` wheel is dynamically linked, so suite 2 needs `nix-ld`.

## Step 3: rules during the run

- **Serialize GPU suites.** When `$DEVICE` is `cuda` or `metal`, run test suites sequentially to avoid device contention.
- **Per-test 30 s budget.** Flag any individual test that exceeds 30 s wall-clock. That is a real bug — production or test — not flakiness.

## Step 4: report

After all suites finish, sum up the results in an actionable report.
