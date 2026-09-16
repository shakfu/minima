BUDGET      := 4000
PROVIDER    ?= openrouter
BIN         := target/debug/minima
INSTALL_DIR := $(HOME)/.local/bin

.PHONY: all build release test lint fmt check run repl live clean install help

all: build

build:
	cargo build

release:
	cargo build --release

test:
	cargo test

# Matches what CI runs. `make fmt` applies what `lint` only reports.
lint:
	cargo fmt --check
	cargo clippy --all-targets -- -D warnings

fmt:
	cargo fmt

check: lint test

# Offline smoke tests: no network, no API key.
run: build
	$(BIN) --mock mock/read-then-answer.json -p "what is this package?"

repl: build
	$(BIN) --mock mock/say-hi.json

# Live smoke test against a real endpoint. Needs a key; scripts/lib.sh says where it looks.
# Runs from a throwaway sandbox, because the model chooses what bash runs.
live: build
	scripts/test_$(PROVIDER).sh

install: release
	@install -d $(INSTALL_DIR)
	@install -m 755 target/release/minima $(INSTALL_DIR)/minima
	@echo "installed minima to $(INSTALL_DIR)"

clean:
	cargo clean

help:
	@echo "build     compile (default)"
	@echo "check     lint + test; the full gate"
	@echo "lint      cargo fmt --check, then clippy with warnings denied"
	@echo "fmt       apply rustfmt"
	@echo "test      unit tests plus the live-path tests (needs python3)"
	@echo "run       one-shot against the mock provider"
	@echo "repl      interactive against the mock provider"
	@echo "live      live suite; PROVIDER=openrouter|openai|anthropic"
	@echo "release   optimised build"
	@echo "install   release build, copied to $(INSTALL_DIR)"
	@echo "clean     remove target/"
