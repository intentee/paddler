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

# -----------------------------------------------------------------------------
# Phony targets
# -----------------------------------------------------------------------------

.PHONY: clean
clean:
	rm -rf esbuild-meta.json
	rm -rf node_modules
	rm -rf static
	rm -rf target

.PHONY: fmt
fmt: node_modules
	./jarmuz-fmt.mjs

.PHONY: jarmuz-static
jarmuz-static: node_modules
	./jarmuz-static.mjs

.PHONY: release
release: node_modules
	./jarmuz-release.mjs

.PHONY: release.cuda
release.cuda: node_modules
	./jarmuz-release.mjs --cuda

.PHONY: release.vulkan
release.vulkan: node_modules
	./jarmuz-release.mjs --vulkan

.PHONY: test
test: jarmuz-static
	cargo test

.PHONY: test.llms
test.llms:
	cargo test --features tests_that_use_llms -- --nocapture

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
