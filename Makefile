.DEFAULT_GOAL := release

RUST_LOG ?= debug

# -----------------------------------------------------------------------------
# Real targets
# -----------------------------------------------------------------------------

package-lock.json: package.json
	npm install --package-lock-only

node_modules: package-lock.json
	npm ci
	touch node_modules

target/debug/paddler: $(shell find paddler/src paddler_bootstrap/src paddler_cli/src paddler_client/src paddler_types/src -name '*.rs')
	cargo build -p paddler_cli

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
clippy: jarmuz-static
	cargo clippy --workspace --all-targets --features web_admin_panel,tests_that_use_llms,tests_that_use_compiled_paddler

.PHONY: fmt
fmt: node_modules
	./jarmuz-fmt.mjs

.PHONY: jarmuz-static
jarmuz-static: node_modules
	./jarmuz-static.mjs

.PHONY: build
build: jarmuz-static
	cargo build -p paddler_cli --features web_admin_panel

.PHONY: build.cuda
build.cuda: jarmuz-static
	cargo build -p paddler_cli --features cuda,web_admin_panel

.PHONY: release
release: jarmuz-static
	cargo build --release -p paddler_cli --features web_admin_panel

.PHONY: release.cuda
release.cuda: jarmuz-static
	cargo build --release -p paddler_cli --features web_admin_panel,cuda

.PHONY: release.vulkan
release.vulkan: jarmuz-static
	cargo build --release -p paddler_cli --features web_admin_panel,vulkan

.PHONY: release.gui
release.gui: jarmuz-static
	cargo build --release -p paddler_gui --features web_admin_panel

PADDLER_TEST_DEVICE ?= cpu

ifeq ($(PADDLER_TEST_DEVICE),cuda)
PADDLER_TEST_DEVICE_FEATURE := ,cuda
PADDLER_TEST_DEVICE_BUILD_FLAGS := --features cuda
else ifeq ($(PADDLER_TEST_DEVICE),metal)
PADDLER_TEST_DEVICE_FEATURE := ,metal
PADDLER_TEST_DEVICE_BUILD_FLAGS := --features metal
else
PADDLER_TEST_DEVICE_FEATURE :=
PADDLER_TEST_DEVICE_BUILD_FLAGS :=
endif

.PHONY: test.all
test.all: test.unit test test.gui

.PHONY: test
test:
	cargo build -p paddler_cli $(PADDLER_TEST_DEVICE_BUILD_FLAGS)
	cargo test -p paddler_tests --no-fail-fast --features tests_that_use_compiled_paddler,tests_that_use_llms$(PADDLER_TEST_DEVICE_FEATURE) -- --nocapture

.PHONY: test.unit
test.unit: jarmuz-static
	cargo test --features web_admin_panel

.PHONY: test.gui
test.gui: target/debug/paddler target/debug/paddler_gui
	cargo test -p paddler_gui_tests --features tests_that_use_compiled_paddler -- --nocapture --test-threads=1

target/debug/paddler_gui: $(shell find paddler_gui/src -name '*.rs')
	cargo build -p paddler_gui

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
