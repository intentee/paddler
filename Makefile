.DEFAULT_GOAL := target/release/paddler

.NOTINTERMEDIATE: target/kev_%/backbone/config.json

.NOTPARALLEL:

RUST_LOG ?= debug

PADDLER_CRATES := paddler_agent paddler_agent_decision paddler_agent_embeddings paddler_agent_pointer_head paddler_agent_runtime paddler_agent_status paddler_agent_text_generation paddler_balancer paddler_bootstrap paddler_cache_dir paddler_cli paddler_client paddler_download_manager paddler_image_decoder paddler_inference_parameters paddler_messaging paddler_model_source paddler_openai_translation paddler_request_registry paddler_state_database paddler_tool_call_validator paddler_typesafe_translation
PADDLER_SOURCES := $(shell find $(addsuffix /src,$(PADDLER_CRATES)) -name '*.rs') $(addsuffix /Cargo.toml,$(PADDLER_CRATES)) Cargo.toml Cargo.lock
FRONTEND_SOURCES := $(shell find resources paddler_client_javascript/src -type f) paddler_client_javascript/package.json $(wildcard jarmuz/*.mjs)
JS_CLIENT_SOURCES := $(shell find paddler_client_javascript/src -type f) paddler_client_javascript/package.json paddler_client_javascript/tsconfig.json
KEV_CHECKPOINT_0_8b := jaredpalmer/kev-0.8b@788ddbdd65715bb03a56788c822f6c632c9a551d
KEV_CHECKPOINT_27b := jaredpalmer/kev-27b@af0e6d551bdc2cc724f3e9d7a8bee1cd4fb8f7bf
KEV_CONVERTER_SOURCES := $(shell find paddler_kev_converter_python/paddler_kev_converter -name '*.py')
QWEN3_5_BASE := Qwen/Qwen3.5-0.8B-Base
QWEN3_5_BASE_REVISION := dc7cdfe2ee4154fa7e30f5b51ca41bfa40174e68
QWEN3_8_BASE := Qwen/Qwen3.8-27B
QWEN3_8_BASE_REVISION := 1d4bf0f2ff6012fd82039f2fa52739d0dd7c60c0
LLAMA_CPP_SOURCE_DIRECTORY = $(dir $(shell cargo metadata --format-version 1 | jq -r '.packages[] | select(.name == "llama-cpp-bindings-sys") | .manifest_path'))llama.cpp
RUN_WITH_TEST_CLUSTER_SOURCES := $(shell find paddler_tests/src paddler_test_cluster_harness/src -name '*.rs') paddler_tests/Cargo.toml paddler_test_cluster_harness/Cargo.toml
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

fixtures/kev_%_parity_reference.json: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter parity-reference --checkpoint $(KEV_CHECKPOINT_$*) --output $(CURDIR)/$@

fixtures/pointer_head_with_f16_tensors.gguf: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter malformed-pointer-head --defect f16-tensors --hidden-size 64 --output $(CURDIR)/$@

fixtures/pointer_head_with_mismatched_projections.gguf: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter malformed-pointer-head --defect mismatched-projections --hidden-size 64 --output $(CURDIR)/$@

fixtures/pointer_head_with_zero_temperature.gguf: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter malformed-pointer-head --defect zero-temperature --hidden-size 64 --output $(CURDIR)/$@

fixtures/pointer_head_without_delimiters.gguf: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter malformed-pointer-head --defect missing-delimiters --hidden-size 64 --output $(CURDIR)/$@

fixtures/pointer_head_without_temperature.gguf: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter malformed-pointer-head --defect missing-temperature --hidden-size 64 --output $(CURDIR)/$@

fixtures/qwen3_5_0_8b_pointer_head_with_plain_text_delimiters.gguf: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter malformed-pointer-head --defect plain-text-delimiters --hidden-size 1024 --output $(CURDIR)/$@

fixtures/qwen3_5_0_8b_synthetic_pointer_head.gguf: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter synthetic-pointer-head --hidden-size 1024 --output $(CURDIR)/$@

fixtures/qwen3_5_0_8b_synthetic_pointer_head_with_wrong_hidden_size.gguf: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter synthetic-pointer-head --hidden-size 512 --output $(CURDIR)/$@

fixtures/qwen3_8_27b_synthetic_pointer_head.gguf: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter synthetic-pointer-head --hidden-size 5120 --output $(CURDIR)/$@

fixtures/qwen3_%_tokenizer_reference.json: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter tokenizer-reference --base $(QWEN3_$*_BASE) --base-revision $(QWEN3_$*_BASE_REVISION) --output $(CURDIR)/$@

fixtures/typesafe_reference.json: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter typesafe-reference --output $(CURDIR)/$@

node_modules: package-lock.json
	npm ci
	touch node_modules

package-lock.json: package.json paddler_client_javascript/package.json
	npm install --package-lock-only

paddler_client_javascript/dist: $(JS_CLIENT_SOURCES) node_modules
	npm --workspace @intentee/paddler-client run build
	touch $@

paddler_kev_converter_python/.venv: paddler_kev_converter_python/poetry.lock
	POETRY_VIRTUALENVS_IN_PROJECT=true poetry -C paddler_kev_converter_python install
	touch $@

paddler_llama_cpp_converter_python/.venv: paddler_llama_cpp_converter_python/poetry.lock
	POETRY_VIRTUALENVS_IN_PROJECT=true poetry -C paddler_llama_cpp_converter_python install
	touch $@

paddler_openai_client_python_test/.venv: paddler_openai_client_python_test/poetry.lock
	POETRY_VIRTUALENVS_IN_PROJECT=true poetry -C paddler_openai_client_python_test install
	touch $@

paddler_typesafe_client_python_test/.venv: paddler_typesafe_client_python_test/poetry.lock
	POETRY_VIRTUALENVS_IN_PROJECT=true poetry -C paddler_typesafe_client_python_test install
	touch $@

$(RUN_WITH_TEST_CLUSTER_BINARY): $(PADDLER_SOURCES) $(RUN_WITH_TEST_CLUSTER_SOURCES)
	cargo build -p paddler_tests --bin run_with_test_cluster --features tests_that_use_llms$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

target/cuda/debug/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build -p paddler_cli --features cuda,web_admin_panel --target-dir target/cuda

target/cuda/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features cuda,web_admin_panel --target-dir target/cuda

target/debug/paddler: $(PADDLER_SOURCES)
	cargo build -p paddler_cli

target/kev_%/backbone/config.json target/kev_%/pointer_head.gguf &: $(KEV_CONVERTER_SOURCES) paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run paddler-kev-converter convert --checkpoint $(KEV_CHECKPOINT_$*) --backbone-directory $(CURDIR)/target/kev_$*/backbone --pointer-head $(CURDIR)/target/kev_$*/pointer_head.gguf

target/kev_%/model.gguf: target/kev_%/backbone/config.json paddler_llama_cpp_converter_python/.venv
	poetry -C paddler_llama_cpp_converter_python run python $(LLAMA_CPP_SOURCE_DIRECTORY)/convert_hf_to_gguf.py $(CURDIR)/target/kev_$*/backbone --no-mtp --outtype bf16 --outfile $(CURDIR)/$@

target/metal/debug/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build -p paddler_cli --features metal,web_admin_panel --target-dir target/metal

target/metal/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features metal,web_admin_panel --target-dir target/metal

target/release/paddler: $(PADDLER_SOURCES) esbuild-meta.json
	cargo build --release -p paddler_cli --features web_admin_panel

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

.PHONY: lint.kev_converter.python
lint.kev_converter.python: paddler_kev_converter_python/.venv
	poetry -C paddler_kev_converter_python run ruff check
	poetry -C paddler_kev_converter_python run ruff format --check
	poetry -C paddler_kev_converter_python run mypy .

.PHONY: lint.openai.python
lint.openai.python: paddler_openai_client_python_test/.venv
	poetry -C paddler_openai_client_python_test run ruff check
	poetry -C paddler_openai_client_python_test run ruff format --check
	poetry -C paddler_openai_client_python_test run mypy .

.PHONY: lint.typesafe.python
lint.typesafe.python: paddler_typesafe_client_python_test/.venv
	poetry -C paddler_typesafe_client_python_test run ruff check
	poetry -C paddler_typesafe_client_python_test run ruff format --check
	poetry -C paddler_typesafe_client_python_test run mypy .

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
test.coverage: esbuild-meta.json node_modules target/kev_0_8b/model.gguf target/kev_0_8b/pointer_head.gguf target/test-model-cards.stamp
	cargo llvm-cov clean --workspace
	NEXTEST_PROFILE=$(NEXTEST_PROFILE) cargo llvm-cov nextest --features tests_that_use_llms,web_admin_panel$(TEST_DEVICE_FEATURE_SUFFIX) --no-report --workspace
	cargo llvm-cov report --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)' --json --output-path target/llvm-cov.json
	cargo llvm-cov report --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)' --lcov --output-path target/lcov.info
	cargo llvm-cov report --no-default-ignore-filename-regex --ignore-filename-regex '$(LLVM_COV_THIRD_PARTY_SOURCES)'
	npx rust-coverage-check target/llvm-cov.json \
		--workspace-root $(CURDIR) \
		--gated paddler_agent=97 \
		--gated paddler_agent_decision=95.99 \
		--gated paddler_agent_embeddings=98.04 \
		--gated paddler_agent_pointer_head=100 \
		--gated paddler_agent_runtime=99 \
		--gated paddler_agent_status=100 \
		--gated paddler_agent_text_generation=96.77 \
		--gated paddler_balancer=99 \
		--gated paddler_bootstrap=100 \
		--gated paddler_cache_dir=100 \
		--gated paddler_cli=99 \
		--gated paddler_cli_tests=94 \
		--gated paddler_client=99 \
		--gated paddler_download_manager=99 \
		--gated paddler_image_decoder=100 \
		--gated paddler_inference_parameters=100 \
		--gated paddler_local_http_fixture=89 \
		--gated paddler_messaging=100 \
		--gated paddler_model_source=99 \
		--gated paddler_openai_response_format_validator=100 \
		--gated paddler_openai_translation=100 \
		--gated paddler_opencode_tests=79 \
		--gated paddler_request_registry=100 \
		--gated paddler_state_database=99 \
		--gated paddler_test_cluster_harness=94 \
		--gated paddler_tests=98 \
		--gated paddler_tool_call_validator=100 \
		--gated paddler_typesafe_translation=100

.PHONY: test.coverage-clean
test.coverage-clean:
	cargo llvm-cov clean --workspace
	rm -rf target/llvm-cov-target
	rm -f target/llvm-cov.json target/lcov.info

.PHONY: test.integration
test.integration: target/kev_0_8b/model.gguf target/kev_0_8b/pointer_head.gguf target/test-model-cards.stamp
	NEXTEST_PROFILE=$(NEXTEST_PROFILE) cargo nextest run -p paddler_tests -p paddler_cli_tests --features tests_that_use_llms$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: test.integration.opencode
test.integration.opencode: target/test-model-cards.stamp
	NEXTEST_PROFILE=$(NEXTEST_PROFILE) cargo nextest run -p paddler_opencode_tests --features tests_that_use_llms,tests_that_use_opencode$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: test.kev_converter.python.llm
test.kev_converter.python.llm: paddler_kev_converter_python/.venv
	PADDLER_KEV_CHECKPOINT=$(KEV_CHECKPOINT_0_8b) poetry -C paddler_kev_converter_python run pytest --cov

.PHONY: test.openai.python.llm
test.openai.python.llm: $(RUN_WITH_TEST_CLUSTER_BINARY) paddler_openai_client_python_test/.venv target/test-model-cards.stamp
	$(RUN_WITH_TEST_CLUSTER_BINARY) qwen3-0-6b -- poetry -C paddler_openai_client_python_test run pytest

.PHONY: test.typesafe.python.llm
test.typesafe.python.llm: $(RUN_WITH_TEST_CLUSTER_BINARY) paddler_typesafe_client_python_test/.venv target/kev_0_8b/model.gguf target/kev_0_8b/pointer_head.gguf
	$(RUN_WITH_TEST_CLUSTER_BINARY) kev-0-8b -- poetry -C paddler_typesafe_client_python_test run pytest

.PHONY: test.unit
test.unit: esbuild-meta.json
	NEXTEST_PROFILE=$(NEXTEST_PROFILE) cargo nextest run --features web_admin_panel$(TEST_DEVICE_FEATURE_SUFFIX) $(TEST_DEVICE_TARGET_DIR)

.PHONY: watch
watch: node_modules
	./jarmuz-watch.mjs
