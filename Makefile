.DEFAULT_GOAL := target/release/paddler

RUST_LOG ?= debug

PADDLER_CRATES := paddler_agent paddler_agent_status paddler_balancer paddler_bootstrap paddler_cache_dir paddler_cli paddler_client paddler_download_manager paddler_gui paddler_image_decoder paddler_inference_parameters paddler_messaging paddler_model_source paddler_request_registry paddler_state_database paddler_tool_call_validator
PADDLER_SOURCES := $(shell find $(addsuffix /src,$(PADDLER_CRATES)) -name '*.rs') $(addsuffix /Cargo.toml,$(PADDLER_CRATES)) Cargo.toml Cargo.lock
FRONTEND_SOURCES := $(shell find resources paddler_client_javascript/src -type f) paddler_client_javascript/package.json $(wildcard jarmuz/*.mjs)
JS_CLIENT_SOURCES := $(shell find paddler_client_javascript/src -type f) paddler_client_javascript/package.json paddler_client_javascript/tsconfig.json
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

esbuild-meta.json: $(FRONTEND_SOURCES) jarmuz-static.mjs tsconfig.json package.json node_modules
	./jarmuz-static.mjs

node_modules: package-lock.json
	npm ci
	touch node_modules

package-lock.json: package.json paddler_client_javascript/package.json
	npm install --package-lock-only

paddler_client_javascript/dist: $(JS_CLIENT_SOURCES) node_modules
	npm --workspace @intentee/paddler-client run build
	touch $@

paddler_client_python/.venv: paddler_client_python/poetry.lock
	POETRY_VIRTUALENVS_IN_PROJECT=true poetry -C paddler_client_python install
	touch $@

paddler_openai_client_python_test/.venv: paddler_openai_client_python_test/poetry.lock
	POETRY_VIRTUALENVS_IN_PROJECT=true poetry -C paddler_openai_client_python_test install
	touch $@

paddler_test_cluster_python/.venv: paddler_test_cluster_python/poetry.lock
	POETRY_VIRTUALENVS_IN_PROJECT=true poetry -C paddler_test_cluster_python install
	touch $@

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

target/test-model-cards.stamp: $(wildcard paddler_test_cluster_harness/src/model_card/*.rs) paddler_test_cluster_harness/tests/every_model_card_resolves_from_the_hugging_face_cache.rs
	cargo test -p paddler_test_cluster_harness --features tests_that_use_llms --test every_model_card_resolves_from_the_hugging_face_cache
	touch $@

target/vulkan/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features vulkan,web_admin_panel --target-dir target/vulkan

.PHONY: build.client.js
build.client.js: paddler_client_javascript/dist

.PHONY: clean
clean:
	rm -rf esbuild-meta.json
	rm -rf node_modules
	rm -rf static
	rm -rf target

.PHONY: clippy
clippy: esbuild-meta.json
	cargo clippy --workspace --all-targets --features web_admin_panel,tests_that_use_llms -- --deny warnings

.PHONY: fmt
fmt: node_modules
	./jarmuz-fmt.mjs

.PHONY: lint.client.python
lint.client.python: paddler_client_python/.venv paddler_test_cluster_python/.venv
	poetry -C paddler_client_python run ruff check
	poetry -C paddler_client_python run ruff format --check
	poetry -C paddler_client_python run mypy .
	poetry -C paddler_test_cluster_python run ruff check
	poetry -C paddler_test_cluster_python run ruff format --check
	poetry -C paddler_test_cluster_python run mypy .

.PHONY: lint.openai.python
lint.openai.python: paddler_openai_client_python_test/.venv
	poetry -C paddler_openai_client_python_test run ruff check
	poetry -C paddler_openai_client_python_test run ruff format --check
	poetry -C paddler_openai_client_python_test run mypy .

.PHONY: test
test: test.client.js test.client.python test.unit test.integration

.PHONY: test.client.js
test.client.js: $(PADDLER_TEST_BINARY) node_modules
	$(PADDLER_BINARY_ENVIRONMENT) npm --workspace @intentee/paddler-client test

.PHONY: test.client.js.coverage
test.client.js.coverage: $(PADDLER_TEST_BINARY) node_modules
	$(PADDLER_BINARY_ENVIRONMENT) npm --workspace @intentee/paddler-client run test:coverage

.PHONY: test.client.js.llm
test.client.js.llm: $(PADDLER_TEST_BINARY) node_modules target/test-model-cards.stamp
	$(PADDLER_BINARY_ENVIRONMENT) npm --workspace @intentee/paddler-client run test:llm

.PHONY: test.client.python
test.client.python: $(PADDLER_TEST_BINARY) paddler_client_python/.venv paddler_test_cluster_python/.venv
	$(PADDLER_BINARY_ENVIRONMENT) poetry -C paddler_test_cluster_python run pytest tests/unit tests/cluster
	$(PADDLER_BINARY_ENVIRONMENT) poetry -C paddler_client_python run pytest tests/unit tests/cluster

.PHONY: test.client.python.coverage
test.client.python.coverage: $(PADDLER_TEST_BINARY) paddler_client_python/.venv paddler_test_cluster_python/.venv
	$(PADDLER_BINARY_ENVIRONMENT) poetry -C paddler_test_cluster_python run pytest --cov
	$(PADDLER_BINARY_ENVIRONMENT) poetry -C paddler_client_python run pytest --cov

.PHONY: test.client.python.llm
test.client.python.llm: $(PADDLER_TEST_BINARY) paddler_client_python/.venv paddler_test_cluster_python/.venv target/test-model-cards.stamp
	$(PADDLER_BINARY_ENVIRONMENT) poetry -C paddler_test_cluster_python run pytest tests/llm
	$(PADDLER_BINARY_ENVIRONMENT) poetry -C paddler_client_python run pytest tests/llm

.PHONY: test.coverage
test.coverage: esbuild-meta.json node_modules target/test-model-cards.stamp
	cargo llvm-cov clean --workspace
	cargo llvm-cov nextest --features tests_that_use_llms,web_admin_panel$(TEST_DEVICE_FEATURE_SUFFIX) --no-report --workspace
	cargo llvm-cov report --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)' --json --output-path target/llvm-cov.json
	cargo llvm-cov report --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)' --lcov --output-path target/lcov.info
	cargo llvm-cov report --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)'
	npx rust-coverage-check target/llvm-cov.json \
		--workspace-root $(CURDIR) \
		--gated paddler_agent=97 \
		--gated paddler_agent_status=100 \
		--gated paddler_balancer=99 \
		--gated paddler_bootstrap=100 \
		--gated paddler_cache_dir=100 \
		--gated paddler_cli=99 \
		--gated paddler_cli_tests=94 \
		--gated paddler_client=99 \
		--gated paddler_download_manager=99 \
		--gated paddler_gui=91 \
		--gated paddler_gui_tests=97 \
		--gated paddler_image_decoder=100 \
		--gated paddler_inference_parameters=100 \
		--gated paddler_local_http_fixture=89 \
		--gated paddler_messaging=100 \
		--gated paddler_model_source=99 \
		--gated paddler_openai_response_format_validator=100 \
		--gated paddler_opencode_tests=79 \
		--gated paddler_request_registry=100 \
		--gated paddler_state_database=99 \
		--gated paddler_test_cluster_harness=94 \
		--gated paddler_tests=98 \
		--gated paddler_tool_call_validator=100

.PHONY: test.coverage-clean
test.coverage-clean:
	cargo llvm-cov clean --workspace
	rm -rf target/llvm-cov-target
	rm -f target/llvm-cov.json target/lcov.info

.PHONY: test.integration
test.integration: target/test-model-cards.stamp
	cargo nextest run -p paddler_tests -p paddler_cli_tests --features tests_that_use_llms$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: test.integration.opencode
test.integration.opencode: target/test-model-cards.stamp
	cargo nextest run -p paddler_opencode_tests --features tests_that_use_llms,tests_that_use_opencode$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: test.openai.python.llm
test.openai.python.llm: $(PADDLER_TEST_BINARY) paddler_openai_client_python_test/.venv target/test-model-cards.stamp
	$(PADDLER_BINARY_ENVIRONMENT) poetry -C paddler_openai_client_python_test run pytest

.PHONY: test.unit
test.unit: esbuild-meta.json
	cargo nextest run --features web_admin_panel$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
