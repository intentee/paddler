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
	cargo build --release -p paddler_gui

.PHONY: test
test: test.unit test.models test.integration

.PHONY: test.models
test.models:
	cargo test -p paddler_model_tests --features tests_that_use_llms -- --nocapture --test-threads=1

.PHONY: test.cuda
test.cuda:
	cargo test -p paddler_model_tests --features tests_that_use_llms,cuda -- --nocapture --test-threads=1

.PHONY: test.unit
test.unit: jarmuz-static
	cargo test --features web_admin_panel

.PHONY: test.integration
test.integration: target/debug/paddler
	@for test_file in paddler_integration_tests/tests/*.rs; do \
		test_name=$$(basename $$test_file .rs); \
		echo "==> cargo test --test $$test_name"; \
		cargo test -p paddler_integration_tests --features tests_that_use_compiled_paddler,tests_that_use_llms --test $$test_name -- --nocapture --test-threads=1 || exit $$?; \
	done

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
