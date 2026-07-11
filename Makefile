.DEFAULT_GOAL := target/release/paddler

RUST_LOG ?= debug

PADDLER_SOURCES := $(shell find paddler_agent/src paddler_balancer/src paddler_bootstrap/src paddler_cache_dir/src paddler_cli/src paddler_client/src paddler_download_manager/src paddler_gui/src paddler_messaging/src paddler_state_conversion/src -name '*.rs')
FRONTEND_SOURCES := $(shell find resources -type f) $(wildcard jarmuz/*.mjs)

TEST_DEVICE ?= cpu

ifeq ($(TEST_DEVICE),cpu)
TEST_DEVICE_FEATURE_SUFFIX :=
TEST_DEVICE_TARGET_DIR :=
else
TEST_DEVICE_FEATURE_SUFFIX := ,$(TEST_DEVICE)
TEST_DEVICE_TARGET_DIR := --target-dir target/$(TEST_DEVICE)
endif

# -----------------------------------------------------------------------------
# Real targets
# -----------------------------------------------------------------------------

esbuild-meta.json: $(FRONTEND_SOURCES) jarmuz-static.mjs tsconfig.json package.json node_modules
	./jarmuz-static.mjs

node_modules: package-lock.json
	npm ci
	touch node_modules

package-lock.json: package.json
	npm install --package-lock-only

target/cuda/debug/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build -p paddler_cli --features cuda,web_admin_panel --target-dir target/cuda

target/cuda/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features cuda,web_admin_panel --target-dir target/cuda

target/cuda/release/paddler_gui: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_gui --features cuda,web_admin_panel --target-dir target/cuda

target/debug/paddler: $(PADDLER_SOURCES)
	cargo build -p paddler_cli

target/metal/debug/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build -p paddler_cli --features metal,web_admin_panel --target-dir target/metal

target/metal/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features metal,web_admin_panel --target-dir target/metal

target/metal/release/paddler_gui: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_gui --features metal,web_admin_panel --target-dir target/metal

target/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features web_admin_panel

target/release/paddler_gui: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_gui --features web_admin_panel

target/vulkan/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features vulkan,web_admin_panel --target-dir target/vulkan

# -----------------------------------------------------------------------------
# Phony targets
# -----------------------------------------------------------------------------

.PHONY: build.client.js
build.client.js: node_modules
	npm --workspace @intentee/paddler-client run build

.PHONY: clean
clean:
	rm -rf esbuild-meta.json
	rm -rf node_modules
	rm -rf static
	rm -rf target

.PHONY: clippy
clippy: esbuild-meta.json
	cargo clippy --workspace --all-targets --features web_admin_panel,tests_that_use_llms

.PHONY: fmt
fmt: node_modules
	./jarmuz-fmt.mjs

.PHONY: test
test: test.client.js test.unit test.integration

.PHONY: test.client.js
test.client.js: node_modules
	npm --workspace @intentee/paddler-client test

.PHONY: test.coverage
test.coverage: esbuild-meta.json node_modules
	cargo llvm-cov clean --profraw-only
	cargo llvm-cov nextest --features tests_that_use_llms,web_admin_panel$(TEST_DEVICE_FEATURE_SUFFIX) --no-report --workspace
	cargo llvm-cov report --json --output-path target/llvm-cov.json
	cargo llvm-cov report --lcov --output-path target/lcov.info
	cargo llvm-cov report
	npx rust-coverage-check target/llvm-cov.json \
		--workspace-root $(CURDIR) \
		--gated paddler_agent=96 \
		--gated paddler_balancer=84 \
		--gated paddler_bootstrap=100 \
		--gated paddler_cache_dir=100 \
		--gated paddler_cli=83 \
		--gated paddler_cli_tests=87 \
		--gated paddler_client=41 \
		--gated paddler_download_manager=99 \
		--gated paddler_gui=13 \
		--gated paddler_messaging=100 \
		--gated paddler_openai_response_format_validator=99 \
		--gated paddler_opencode_tests=93 \
		--gated paddler_test_cluster_harness=67

.PHONY: test.coverage-clean
test.coverage-clean:
	cargo llvm-cov clean --workspace
	rm -rf target/llvm-cov-target
	rm -f target/llvm-cov.json target/lcov.info

.PHONY: test.integration
test.integration:
	cargo nextest run -p paddler_tests -p paddler_cli_tests --features tests_that_use_llms$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: test.integration.opencode
test.integration.opencode:
	cargo nextest run -p paddler_opencode_tests --features tests_that_use_llms,tests_that_use_opencode$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: test.unit
test.unit: esbuild-meta.json
	cargo nextest run --features web_admin_panel$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
