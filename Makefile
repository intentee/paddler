.DEFAULT_GOAL := target/release/paddler

.NOTPARALLEL:

CARGO_HOME ?= $(HOME)/.cargo
HOST_TUPLE := $(shell rustc --print host-tuple)
RUST_LOG ?= debug
RUST_SYSROOT := $(shell rustc --print sysroot)

PADDLER_CRATES := paddler_agent paddler_agent_decision paddler_agent_embeddings paddler_agent_runtime paddler_agent_status paddler_agent_text_generation paddler_balancer paddler_balancer_dashboard_assets paddler_balancer_openai_service paddler_balancer_state_database paddler_balancer_typesafe_service paddler_bootstrap paddler_cache_dir paddler_cli paddler_client paddler_download_manager paddler_error_chain paddler_frame_writer paddler_image_decoder paddler_inference_parameters paddler_messaging paddler_model_source paddler_openai_translation paddler_outbox paddler_request_registry paddler_sha256_hex paddler_spawn_resuming_panic paddler_tool_call_validator paddler_typesafe_translation
PADDLER_SOURCES := $(shell find $(addsuffix /src,$(PADDLER_CRATES)) -name '*.rs') $(wildcard $(addsuffix /build.rs,$(PADDLER_CRATES))) $(wildcard $(addsuffix /build/*.rs,$(PADDLER_CRATES))) $(addsuffix /Cargo.toml,$(PADDLER_CRATES)) Cargo.toml Cargo.lock
FRONTEND_SOURCES := $(shell find resources paddler_client_javascript/src -type f) paddler_client_javascript/package.json $(wildcard jarmuz/*.mjs)
JS_CLIENT_SOURCES := $(shell find paddler_client_javascript/src -type f) paddler_client_javascript/package.json paddler_client_javascript/tsconfig.json
RUN_WITH_TEST_CLUSTER_CRATES := paddler_balancer_desired_state_fixture paddler_cluster_harness paddler_local_http_fixture paddler_model_card_fixture
RUN_WITH_TEST_CLUSTER_SOURCES := $(shell find $(addsuffix /src,$(RUN_WITH_TEST_CLUSTER_CRATES)) -name '*.rs') $(addsuffix /Cargo.toml,$(RUN_WITH_TEST_CLUSTER_CRATES))
LLVM_COV_THIRD_PARTY_SOURCES := /\.cargo/(registry|git)/|/\.rustup/toolchains/|^/rustc/|^/nix/store/|^$(CURDIR)/target/

TEST_DEVICE ?= cpu

ifeq ($(TEST_DEVICE),cpu)
NEXTEST_PROFILE := cpu
PADDLER_TEST_BINARY := target/debug/paddler
RUN_WITH_TEST_CLUSTER_BINARY := target/debug/run_with_test_cluster
TEST_DEVICE_FEATURE_SUFFIX :=
TEST_DEVICE_TARGET_DIR :=
else
NEXTEST_PROFILE := default
PADDLER_TEST_BINARY := target/$(TEST_DEVICE)/debug/paddler
RUN_WITH_TEST_CLUSTER_BINARY := target/$(TEST_DEVICE)/debug/run_with_test_cluster
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

paddler_balancer_openai_service_python_tests/.venv: paddler_balancer_openai_service_python_tests/poetry.lock
	POETRY_VIRTUALENVS_IN_PROJECT=true poetry -C paddler_balancer_openai_service_python_tests install
	touch $@

paddler_balancer_typesafe_service_python_tests/.venv: paddler_balancer_typesafe_service_python_tests/poetry.lock
	POETRY_VIRTUALENVS_IN_PROJECT=true poetry -C paddler_balancer_typesafe_service_python_tests install
	touch $@

paddler_client_javascript/dist: $(JS_CLIENT_SOURCES) node_modules
	npm --workspace @intentee/paddler-client run build
	touch $@

$(RUN_WITH_TEST_CLUSTER_BINARY): $(PADDLER_SOURCES) $(RUN_WITH_TEST_CLUSTER_SOURCES)
	cargo build -p paddler_cluster_harness --bin run_with_test_cluster --features tests_that_use_llms$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

target/cuda/debug/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build -p paddler_cli --features cuda,web_admin_panel --target-dir target/cuda

target/cuda/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features cuda,web_admin_panel --target-dir target/cuda

target/debug/paddler: $(PADDLER_SOURCES)
	cargo build -p paddler_cli

target/metal/debug/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build -p paddler_cli --features metal,web_admin_panel --target-dir target/metal

target/metal/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features metal,web_admin_panel --target-dir target/metal

target/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features web_admin_panel

target/test-model-cards.stamp: $(wildcard paddler_model_card_fixture/src/*.rs) paddler_model_card_fixture/tests/every_model_card_resolves_from_the_hugging_face_cache.rs
	cargo test -p paddler_model_card_fixture --features tests_that_use_llms --test every_model_card_resolves_from_the_hugging_face_cache
	touch $@

target/vulkan/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features vulkan,web_admin_panel --target-dir target/vulkan

.PHONY: build.client.js
build.client.js: paddler_client_javascript/dist

.PHONY: check.flake
check.flake:
	nix flake check --print-build-logs

.PHONY: clean
clean:
	rm -rf esbuild-meta.json
	rm -rf node_modules
	rm -rf static
	rm -rf target

.PHONY: clippy
clippy: esbuild-meta.json
	cargo clippy --workspace --all-targets --features web_admin_panel,tests_that_use_llms,tests_that_use_opencode

.PHONY: fmt
fmt: node_modules
	./jarmuz-fmt.mjs

.PHONY: heisenberg.check
heisenberg.check: esbuild-meta.json
	cargo fetch
	heisenberg check \
		--cargo-home "$(CARGO_HOME)" \
		--sysroot "$(RUST_SYSROOT)" \
		--configuration $(HOST_TUPLE) \
		--workspace "$(CURDIR)"

.PHONY: lint.openai.python
lint.openai.python: paddler_balancer_openai_service_python_tests/.venv
	poetry -C paddler_balancer_openai_service_python_tests run mypy .

.PHONY: lint.typesafe.python
lint.typesafe.python: paddler_balancer_typesafe_service_python_tests/.venv
	poetry -C paddler_balancer_typesafe_service_python_tests run mypy .

.PHONY: test
test: test.client.js test.unit test.integration

.PHONY: test.client.js
test.client.js: $(PADDLER_TEST_BINARY) node_modules
	$(PADDLER_BINARY_ENVIRONMENT) npm --workspace @intentee/paddler-client test

.PHONY: test.client.js.coverage
test.client.js.coverage: $(PADDLER_TEST_BINARY) node_modules
	$(PADDLER_BINARY_ENVIRONMENT) npm --workspace @intentee/paddler-client run test:coverage

.PHONY: test.client.js.llm
test.client.js.llm: $(PADDLER_TEST_BINARY) node_modules target/test-model-cards.stamp
	$(PADDLER_BINARY_ENVIRONMENT) npm --workspace @intentee/paddler-client run test:llm

.PHONY: test.coverage
test.coverage: esbuild-meta.json target/test-model-cards.stamp
	cargo llvm-cov clean --workspace
	NEXTEST_PROFILE=$(NEXTEST_PROFILE) cargo llvm-cov nextest --features tests_that_use_llms,tests_that_use_opencode,web_admin_panel$(TEST_DEVICE_FEATURE_SUFFIX) --include-build-script --no-report --workspace
	cargo llvm-cov report --include-build-script --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)' --json --output-path target/llvm-cov.json
	cargo llvm-cov report --include-build-script --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)' --lcov --output-path target/lcov.info
	cargo llvm-cov report --include-build-script --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)'
	cargo fetch
	heisenberg coverage-check \
		--cargo-home "$(CARGO_HOME)" \
		--sysroot "$(RUST_SYSROOT)" \
		--configuration $(HOST_TUPLE) \
		--workspace "$(CURDIR)" \
		--coverage-export "$(CURDIR)/target/llvm-cov.json"

.PHONY: test.coverage-clean
test.coverage-clean:
	cargo llvm-cov clean --workspace
	rm -rf target/llvm-cov-target
	rm -f target/llvm-cov.json target/lcov.info

.PHONY: test.integration
test.integration: target/test-model-cards.stamp
	NEXTEST_PROFILE=$(NEXTEST_PROFILE) cargo nextest run --workspace --features tests_that_use_llms$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: test.integration.opencode
test.integration.opencode: target/test-model-cards.stamp
	NEXTEST_PROFILE=$(NEXTEST_PROFILE) cargo nextest run -p paddler_balancer_openai_service_tests --features tests_that_use_llms,tests_that_use_opencode$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR) -E 'test(=opencode_completes_tool_enabled_chat_request::opencode_completes_tool_enabled_chat_request)'

.PHONY: test.openai.python.llm
test.openai.python.llm: $(RUN_WITH_TEST_CLUSTER_BINARY) paddler_balancer_openai_service_python_tests/.venv target/test-model-cards.stamp
	$(RUN_WITH_TEST_CLUSTER_BINARY) qwen3-0-6b -- poetry -C paddler_balancer_openai_service_python_tests run pytest

.PHONY: test.typesafe.python.llm
test.typesafe.python.llm: $(RUN_WITH_TEST_CLUSTER_BINARY) paddler_balancer_typesafe_service_python_tests/.venv target/test-model-cards.stamp
	$(RUN_WITH_TEST_CLUSTER_BINARY) kev-0-8b -- poetry -C paddler_balancer_typesafe_service_python_tests run pytest

.PHONY: test.unit
test.unit: esbuild-meta.json
	NEXTEST_PROFILE=$(NEXTEST_PROFILE) cargo nextest run --features web_admin_panel$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
