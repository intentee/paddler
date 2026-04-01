.venv: poetry.lock
	poetry sync
	touch .venv

poetry.lock: pyproject.toml
	poetry lock
	touch poetry.lock

.PHONY: check
check: lint test

.PHONY: lint
lint: mypy pyright ruff

.PHONY: mypy
mypy: .venv
	poetry run mypy paddler_client tests

.PHONY: pyright
pyright: .venv
	poetry run pyright

.PHONY: ruff
ruff: .venv
	ruff check paddler_client tests

.PHONY: fmt
fmt:
	ruff format paddler_client tests
	ruff check --fix paddler_client tests

.PHONY: test
test: .venv
	poetry run pytest
