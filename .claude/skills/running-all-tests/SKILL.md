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

`$DEVICE` selects the Rust integration suite variant in Step 2. The other four suites don't take a device feature.

## Step 2: run the five suites

Copy this checklist and tick each item as the suite completes:

```
- [ ] JS client
- [ ] Python client
- [ ] Rust unit
- [ ] Rust integration
- [ ] paddler_gui
```

| # | Suite            | Inner command                                                                                                                     | Working dir              |
|---|------------------|-----------------------------------------------------------------------------------------------------------------------------------|--------------------------|
| 1 | JS client        | `make test.client.js`                                                                                                             | repo root                |
| 2 | Python client    | NixOS: `poetry run pytest`, `ruff`, `poetry run mypy"`. Every other OS: `poetry run pytest`, `poetry run ruff`, `poetry run mypy` | `paddler_client_python/` |
| 3 | Rust unit        | `make test.unit`                                                                                                                  | repo root                |
| 4 | Rust integration | `make test.integration` (cpu) / `make test.integration.cuda` / `make test.integration.metal` — pick by `$DEVICE`                  | repo root                |
| 5 | paddler_gui      | `cargo test -p paddler_gui --features web_admin_panel`                                                                            | `paddler_gui/`           |

Run them in this order. Cheap suites (1, 3, 4) surface bugs quickly; the heavy GPU-bound suites (2, 5) come last.

## Step 3: rules during the run

- **Serialize GPU suites.** When `$DEVICE` is `cuda` or `metal`, run test suites sequentially to avoid device contention.
- **Per-test 30 s budget.** Flag any individual test that exceeds 30 s wall-clock. That is a real bug — production or test — not flakiness.

## Step 4: report

After all suites finish, sum up the results in an actionable report.
