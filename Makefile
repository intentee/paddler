.DEFAULT_GOAL := target/release/paddler

RUST_LOG ?= debug

PADDLER_SOURCES := $(shell find paddler_agent/src paddler_balancer/src paddler_bootstrap/src paddler_cache_dir/src paddler_cli/src paddler_client/src paddler_download_manager/src paddler_gui/src paddler_image_decoder/src paddler_messaging/src paddler_state_conversion/src paddler_tool_call_validator/src -name '*.rs')
FRONTEND_SOURCES := $(shell find resources paddler_client_javascript/src -type f) paddler_client_javascript/package.json $(wildcard jarmuz/*.mjs)
LLVM_COV_THIRD_PARTY_SOURCES := /\.cargo/(registry|git)/|/\.rustup/toolchains/|^/rustc/|^/nix/store/|^$(CURDIR)/target/

TEST_DEVICE ?= cpu

ifeq ($(TEST_DEVICE),cpu)
PADDLER_TEST_BINARY := target/debug/paddler
TEST_DEVICE_FEATURE_SUFFIX :=
TEST_DEVICE_TARGET_DIR :=
else
PADDLER_TEST_BINARY := target/$(TEST_DEVICE)/debug/paddler
TEST_DEVICE_FEATURE_SUFFIX := ,$(TEST_DEVICE)
TEST_DEVICE_TARGET_DIR := --target-dir target/$(TEST_DEVICE)
endif

PADDLER_BINARY_ENVIRONMENT := PADDLER_BINARY=$(CURDIR)/$(PADDLER_TEST_BINARY)

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

paddler_client_python/.venv: paddler_client_python/poetry.lock
	cd paddler_client_python && POETRY_VIRTUALENVS_IN_PROJECT=true poetry install
	touch paddler_client_python/.venv

paddler_openai_client_python_test/.venv: paddler_openai_client_python_test/poetry.lock
	cd paddler_openai_client_python_test && POETRY_VIRTUALENVS_IN_PROJECT=true poetry install
	touch paddler_openai_client_python_test/.venv

paddler_test_cluster_python/.venv: paddler_test_cluster_python/poetry.lock
	cd paddler_test_cluster_python && POETRY_VIRTUALENVS_IN_PROJECT=true poetry install
	touch paddler_test_cluster_python/.venv

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

.PHONY: lint.client.python
lint.client.python: paddler_client_python/.venv paddler_test_cluster_python/.venv
	cd paddler_client_python && poetry run ruff check && poetry run ruff format --check && poetry run mypy .
	cd paddler_test_cluster_python && poetry run ruff check && poetry run ruff format --check && poetry run mypy .

.PHONY: lint.openai.python
lint.openai.python: paddler_openai_client_python_test/.venv
	cd paddler_openai_client_python_test && poetry run ruff check && poetry run ruff format --check && poetry run mypy .

.PHONY: test
test: test.client.js test.client.python test.unit test.integration

.PHONY: test.client.js
test.client.js: $(PADDLER_TEST_BINARY) node_modules
	$(PADDLER_BINARY_ENVIRONMENT) npm --workspace @intentee/paddler-client test

.PHONY: test.client.js.coverage
test.client.js.coverage: $(PADDLER_TEST_BINARY) node_modules
	$(PADDLER_BINARY_ENVIRONMENT) npm --workspace @intentee/paddler-client run test:coverage

.PHONY: test.client.js.llm
test.client.js.llm: $(PADDLER_TEST_BINARY) node_modules
	$(PADDLER_BINARY_ENVIRONMENT) npm --workspace @intentee/paddler-client run test:llm

.PHONY: test.client.python
test.client.python: $(PADDLER_TEST_BINARY) paddler_client_python/.venv paddler_test_cluster_python/.venv
	cd paddler_test_cluster_python && $(PADDLER_BINARY_ENVIRONMENT) poetry run pytest tests/unit tests/cluster
	cd paddler_client_python && $(PADDLER_BINARY_ENVIRONMENT) poetry run pytest tests/unit tests/cluster

.PHONY: test.client.python.coverage
test.client.python.coverage: $(PADDLER_TEST_BINARY) paddler_client_python/.venv paddler_test_cluster_python/.venv
	cd paddler_test_cluster_python && $(PADDLER_BINARY_ENVIRONMENT) poetry run pytest --cov
	cd paddler_client_python && $(PADDLER_BINARY_ENVIRONMENT) poetry run pytest --cov

.PHONY: test.client.python.llm
test.client.python.llm: $(PADDLER_TEST_BINARY) paddler_client_python/.venv paddler_test_cluster_python/.venv
	cd paddler_test_cluster_python && $(PADDLER_BINARY_ENVIRONMENT) poetry run pytest tests/llm
	cd paddler_client_python && $(PADDLER_BINARY_ENVIRONMENT) poetry run pytest tests/llm

.PHONY: test.coverage
test.coverage: esbuild-meta.json node_modules
	cargo llvm-cov clean --workspace
	cargo llvm-cov nextest --features tests_that_use_llms,web_admin_panel$(TEST_DEVICE_FEATURE_SUFFIX) --no-report --workspace
	cargo llvm-cov report --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)' --json --output-path target/llvm-cov.json
	cargo llvm-cov report --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)' --lcov --output-path target/lcov.info
	cargo llvm-cov report --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)'
	npx rust-coverage-check target/llvm-cov.json \
		--workspace-root $(CURDIR) \
		--gated paddler_agent=97 \
		--gated paddler_balancer=97 \
		--gated paddler_bootstrap=100 \
		--gated paddler_cache_dir=100 \
		--gated paddler_cli=87 \
		--gated paddler_cli_tests=89 \
		--gated paddler_client=97 \
		--gated paddler_download_manager=99 \
		--gated paddler_gui=37 \
		--gated paddler_image_decoder=100 \
		--gated paddler_local_http_fixture=89 \
		--gated paddler_messaging=100 \
		--gated paddler_openai_response_format_validator=99 \
		--gated paddler_opencode_tests=76 \
		--gated paddler_test_cluster_harness=94 \
		--gated paddler_tests=88 \
		--gated paddler_tool_call_validator=100

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

.PHONY: test.openai.python.llm
test.openai.python.llm: $(PADDLER_TEST_BINARY) paddler_openai_client_python_test/.venv
	cd paddler_openai_client_python_test && $(PADDLER_BINARY_ENVIRONMENT) poetry run pytest

.PHONY: test.unit
test.unit: esbuild-meta.json
	cargo nextest run --features web_admin_panel$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
