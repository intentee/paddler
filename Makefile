.DEFAULT_GOAL := target/release/paddler

RUST_LOG ?= debug

PADDLER_CLI_SOURCES := $(shell find paddler/src paddler_bootstrap/src paddler_cli/src paddler_client/src paddler_types/src -name '*.rs')
PADDLER_GUI_SOURCES := $(shell find paddler/src paddler_bootstrap/src paddler_gui/src paddler_types/src -name '*.rs')
FRONTEND_SOURCES := $(shell find resources -type f) $(wildcard jarmuz/*.mjs)

# -----------------------------------------------------------------------------
# Real targets
# -----------------------------------------------------------------------------

package-lock.json: package.json
	npm install --package-lock-only

node_modules: package-lock.json
	npm ci
	touch node_modules

esbuild-meta.json: $(FRONTEND_SOURCES) jarmuz-static.mjs tsconfig.json package.json node_modules
	./jarmuz-static.mjs

target/debug/paddler: $(PADDLER_CLI_SOURCES)
	cargo build -p paddler_cli

target/release/paddler: $(PADDLER_CLI_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features web_admin_panel

target/cuda/debug/paddler: $(PADDLER_CLI_SOURCES) esbuild-meta.json
	cargo build -p paddler_cli --features cuda,web_admin_panel --target-dir target/cuda

target/cuda/release/paddler: $(PADDLER_CLI_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features cuda,web_admin_panel --target-dir target/cuda

target/metal/debug/paddler: $(PADDLER_CLI_SOURCES) esbuild-meta.json
	cargo build -p paddler_cli --features metal,web_admin_panel --target-dir target/metal

target/metal/release/paddler: $(PADDLER_CLI_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features metal,web_admin_panel --target-dir target/metal

target/vulkan/release/paddler: $(PADDLER_CLI_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features vulkan,web_admin_panel --target-dir target/vulkan

target/release/paddler_gui: $(PADDLER_GUI_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_gui --features web_admin_panel

# -----------------------------------------------------------------------------
# Phony targets
# -----------------------------------------------------------------------------

.PHONY: clean
clean:
	rm -rf esbuild-meta.json
	rm -rf node_modules
	rm -rf static
	rm -rf target

.PHONY: clippy
clippy: esbuild-meta.json
	cargo clippy --workspace --all-targets --features web_admin_panel,tests_that_use_llms,tests_that_use_compiled_paddler

.PHONY: fmt
fmt: node_modules
	./jarmuz-fmt.mjs

.PHONY: test
test: test.unit test.integration

.PHONY: test.integration
test.integration: target/debug/paddler
	cargo test -p paddler_tests --features tests_that_use_compiled_paddler,tests_that_use_llms

.PHONY: test.integration.cuda
test.integration.cuda: target/cuda/debug/paddler
	PADDLER_BINARY_PATH=../target/cuda/debug/paddler PADDLER_TEST_DEVICE=cuda cargo test -p paddler_tests --features cuda,tests_that_use_compiled_paddler,tests_that_use_llms

.PHONY: test.integration.metal
test.integration.metal: target/metal/debug/paddler
	PADDLER_BINARY_PATH=../target/metal/debug/paddler PADDLER_TEST_DEVICE=metal cargo test -p paddler_tests --features metal,tests_that_use_compiled_paddler,tests_that_use_llms

.PHONY: test.unit
test.unit: esbuild-meta.json
	cargo test --features web_admin_panel

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
