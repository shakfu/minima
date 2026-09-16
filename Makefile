# minima: build, test, and the line budget the frozen scope rests on.
# `make check` is the whole gate. See README.md before adding a target.

BUDGET   := 4000
PROVIDER ?= openrouter
BIN      := target/debug/minima

.PHONY: all build release test lint fmt budget check run repl live clean help

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

# The rule that keeps the scope frozen: a new feature displaces an old one rather than
# accumulating. This target is authoritative; README.md quotes the number.
budget:
	@n=$$(find src -name '*.rs' -exec cat {} + | wc -l | tr -d ' '); \
	 printf '%s / %s lines in src/\n' "$$n" "$(BUDGET)"; \
	 [ "$$n" -le "$(BUDGET)" ] || { \
	   echo "over budget: amend README.md's scope table, or delete something"; exit 1; }

check: lint test budget

# Offline smoke tests: no network, no API key.
run: build
	$(BIN) --mock mock/read-then-answer.json -p "what is this package?"

repl: build
	$(BIN) --mock mock/say-hi.json

# Live smoke test against a real endpoint. Needs a key; scripts/lib.sh says where it looks.
# Runs from a throwaway sandbox, because the model chooses what bash runs.
live: build
	scripts/test_$(PROVIDER).sh

clean:
	cargo clean

help:
	@echo "build     compile (default)"
	@echo "check     lint + test + budget; the full gate"
	@echo "lint      cargo fmt --check, then clippy with warnings denied"
	@echo "fmt       apply rustfmt"
	@echo "test      unit tests plus the live-path tests (needs python3)"
	@echo "budget    fail if src/ exceeds $(BUDGET) lines"
	@echo "run       one-shot against the mock provider"
	@echo "repl      interactive against the mock provider"
	@echo "live      live suite; PROVIDER=openrouter|openai|anthropic"
	@echo "release   optimised build"
	@echo "clean     remove target/"
