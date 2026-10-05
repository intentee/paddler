---
name: running-coverage
description: Runs every test suite in the paddler workspace on the fastest available device, and produces code coverage report. Use when the user asks to run the code coverage, or to check the coverage.
---

# Running the code coverage

Run every test suite in the workspace, picking the fastest compiled device backend for the host, then report the workspace code coverage. 

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

## Step 2: run the code coverage

Copy this checklist and tick each item as the suite completes:

`TEST_DEVICE=$DEVICE make test.coverage`

## Step 3: rules during the run

- **Serialize GPU suites.** When `$DEVICE` is `cuda` or `metal`, run test suites sequentially to avoid device contention.
- **Per-test 30 s budget.** Flag any individual test that exceeds 30 s wall-clock. That is a real bug — production or test — not flakiness.

## Step 4: report

After the coverage suite finishes, sum up the results in an actionable report.
