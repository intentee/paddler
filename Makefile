.DEFAULT_GOAL := target/release/paddler

RUST_LOG ?= debug

COVERAGE_PACKAGES := -p paddler_cache_dir -p paddler_download_manager
PADDLER_SOURCES := $(shell find paddler/src paddler_bootstrap/src paddler_cache_dir/src paddler_cli/src paddler_client/src paddler_download_manager/src paddler_gui/src paddler_types/src -name '*.rs')
FRONTEND_SOURCES := $(shell find resources -type f) $(wildcard jarmuz/*.mjs)

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
	cargo clippy --workspace --all-targets --features web_admin_panel,tests_that_use_llms,tests_that_use_compiled_paddler,tests_that_use_in_process_cluster

.PHONY: coverage
coverage: node_modules
	cargo llvm-cov clean --workspace
	cargo llvm-cov $(COVERAGE_PACKAGES) --no-report
	cargo llvm-cov report --json --output-path target/llvm-cov.json
	cargo llvm-cov report --lcov --output-path target/lcov.info
	cargo llvm-cov report
	npx rust-coverage-check target/llvm-cov.json \
		--workspace-root $(CURDIR) \
		--gated paddler_cache_dir=100 \
		--gated paddler_download_manager=99

.PHONY: coverage-clean
coverage-clean:
	cargo llvm-cov clean --workspace
	rm -rf target/llvm-cov-target
	rm -f target/llvm-cov.json target/lcov.info

.PHONY: coverage-report
coverage-report:
	cargo llvm-cov $(COVERAGE_PACKAGES) --html

.PHONY: fmt
fmt: node_modules
	./jarmuz-fmt.mjs

.PHONY: test
test: test.client.js test.unit test.integration

.PHONY: test.client.js
test.client.js: node_modules
	npm --workspace @intentee/paddler-client test

.PHONY: test.integration
test.integration: target/debug/paddler
	cargo test -p paddler_tests --features tests_that_use_compiled_paddler,tests_that_use_in_process_cluster,tests_that_use_llms

.PHONY: test.integration.cuda
test.integration.cuda: target/cuda/debug/paddler
	PADDLER_BINARY_PATH=../target/cuda/debug/paddler PADDLER_TEST_DEVICE=cuda cargo test --target-dir target/cuda -p paddler_tests --features cuda,tests_that_use_compiled_paddler,tests_that_use_in_process_cluster,tests_that_use_llms

.PHONY: test.integration.metal
test.integration.metal: target/metal/debug/paddler
	PADDLER_BINARY_PATH=../target/metal/debug/paddler PADDLER_TEST_DEVICE=metal cargo test --target-dir target/metal -p paddler_tests --features metal,tests_that_use_compiled_paddler,tests_that_use_in_process_cluster,tests_that_use_llms

.PHONY: test.unit
test.unit: esbuild-meta.json
	cargo test --features web_admin_panel

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
