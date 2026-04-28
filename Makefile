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
clippy: frontend
	cargo clippy --workspace --all-targets --features web_admin_panel,tests_that_use_llms,tests_that_use_compiled_paddler

.PHONY: fmt
fmt: node_modules
	./jarmuz-fmt.mjs

.PHONY: frontend
frontend: node_modules
	./jarmuz-static.mjs

.PHONY: build
build: frontend
	cargo build -p paddler_cli --features web_admin_panel

.PHONY: build.cuda
build.cuda: frontend
	cargo build -p paddler_cli --features cuda,web_admin_panel

.PHONY: release
release: frontend
	cargo build --release -p paddler_cli --features web_admin_panel

.PHONY: release.cuda
release.cuda: frontend
	cargo build --release -p paddler_cli --features web_admin_panel,cuda

.PHONY: release.vulkan
release.vulkan: frontend
	cargo build --release -p paddler_cli --features web_admin_panel,vulkan

.PHONY: release.gui
release.gui: frontend
	cargo build --release -p paddler_gui --features web_admin_panel

.PHONY: test
test: test.unit test.integration

.PHONY: test.integration
test.integration:
	cargo test -p paddler_tests --features tests_that_use_compiled_paddler,tests_that_use_llms

.PHONY: test.integration.cuda
test.integration.cuda:
	PADDLER_TEST_DEVICE=cuda cargo test -p paddler_tests --features cuda,tests_that_use_compiled_paddler,tests_that_use_llms

.PHONY: test.integration.metal
test.integration.metal:
	PADDLER_TEST_DEVICE=metal cargo test -p paddler_tests --features metal,tests_that_use_compiled_paddler,tests_that_use_llms

.PHONY: test.unit
test.unit: frontend
	cargo test --features web_admin_panel

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
