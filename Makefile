.DEFAULT_GOAL := build

RUST_LOG ?= debug

# -----------------------------------------------------------------------------
# Real targets
# -----------------------------------------------------------------------------

package-lock.json: package.json
	npm install --package-lock-only

node_modules: package-lock.json
	npm install --from-lockfile
	touch node_modules

# -----------------------------------------------------------------------------
# Phony targets
# -----------------------------------------------------------------------------

.PHONY: build
build: node_modules
	./jarmuz-release.mjs

.PHONY: clean
clean:
	rm -rf esbuild-meta.json
	rm -rf node_modules
	rm -rf static
	rm -rf target

.PHONY: fmt
fmt: node_modules
	./jarmuz-fmt.mjs
	$(MAKE) -C tests/integration_tests fmt

.PHONY: integration_tests
integration_tests:
	cargo build
	$(MAKE) -C tests/integration_tests test

.PHONY: shell_tests
shell_tests:
	bash tests/scripts/test-model-detection.sh

.PHONY: test
test: integration_tests
	cargo test

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
