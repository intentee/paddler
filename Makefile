.DEFAULT_GOAL := release

RUST_LOG ?= debug

# -----------------------------------------------------------------------------
# Real targets
# -----------------------------------------------------------------------------

package-lock.json: package.json
	npm install --package-lock-only

node_modules: package-lock.json
	npm install --from-lockfile
	touch node_modules

target/debug/paddler: $(shell find paddler/src paddler_types/src paddler_client/src -name '*.rs')
	cargo build -p paddler

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
	cargo clippy --workspace --all-targets --features web_admin_panel

.PHONY: fmt
fmt: node_modules
	./jarmuz-fmt.mjs

.PHONY: jarmuz-static
jarmuz-static: node_modules
	./jarmuz-static.mjs

.PHONY: release
release: jarmuz-static
	cargo build --release -p paddler --features web_admin_panel

.PHONY: release.cuda
release.cuda: jarmuz-static
	cargo build --release -p paddler --features web_admin_panel,cuda

.PHONY: release.vulkan
release.vulkan: jarmuz-static
	cargo build --release -p paddler --features web_admin_panel,vulkan

.PHONY: build
build: jarmuz-static
	cargo build -p paddler --features web_admin_panel

.PHONY: test
test: test.unit test.models test.integration

.PHONY: test.models
test.models:
	timeout 300 cargo test -p paddler_model_tests --features tests_that_use_llms -- --nocapture --test-threads=1

.PHONY: test.cuda
test.cuda:
	timeout 1800 cargo test -p paddler_model_tests --features tests_that_use_llms,cuda -- --nocapture --test-threads=1

.PHONY: test.unit
test.unit: jarmuz-static
	timeout 300 cargo test --features web_admin_panel

.PHONY: test.integration
test.integration: target/debug/paddler
	timeout 300 cargo test -p paddler_integration_tests --features tests_that_use_compiled_paddler,tests_that_use_llms -- --nocapture --test-threads=1

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
