# minima

A minimal coding agent harness with a tiny feature set.

`minima` was created to test whether a usable agent harness can fits into 3,000 lines when the ecosystem carries the capabilities. It was inspired by by [hax](https://github.com/OleksandrChekhovskyi/hax).

```sh
% minima --help
A minimal coding agent for the terminal, with a frozen feature set

Usage: minima [OPTIONS]

Options:
  -p, --prompt <TEXT>   Headless: answer this prompt, print the result, exit
      --provider <ID>   Which provider to talk to: fixes the endpoint, the wire format and the key variable. Left out, minima takes the first provider whose key variable is set [env: MINIMA_PROVIDER=]
      --model <ID>      Left out, minima reuses the model last used with this provider [env: MINIMA_MODEL=]
      --base-url <URL>  Override the provider's endpoint, for a local server or a gateway. Never changes the wire format: a different shape is a different provider, not a different address [env: MINIMA_BASE_URL=]
      --api-key <KEY>   Overrides the provider's key variable [env: MINIMA_API_KEY]
      --no-color        Print without colour. Colour is off anyway when stdout is not a terminal, or when NO_COLOR is set
      --context <N>     Context window in tokens. Falls back to the cached value for the model [env: MINIMA_CONTEXT=]
      --mock <PATH>     Replay a scripted JSON stream instead of calling the network
      --max-turns <N>   Refuse to keep going after this many provider round-trips in one user turn [default: 32]
      --refresh-models  Re-fetch the model list even if the cache is fresh
  -h, --help            Print help
  -V, --version         Print version
```


## Scope

| Dimension | Current Status |
| --- | --- |
| Wire formats | 3: openai-chat, openai-responses, anthropic-messages |
| Providers | a fixed registry; `--provider` names one, or the environment picks |
| Tools | 4: `read`, `write`, `edit`, `bash` |
| Entry modes | 2: interactive REPL, headless `-p` |
| Config | flags, environment, and one cache file |
| Colour | on for a terminal, off for a pipe; `--no-color` and `NO_COLOR` |
| Persistence | model list cache, prompt history, last model per provider |
| Test provider | 1: `mock`, replaying a JSON script |

Each registry entry fixes a base URL, a dialect and a key variable, so `--provider` and `--model` are the whole selection, and both have a fallback:

- No `--provider`: the first entry below whose key variable is set. Local servers carry no key and are never autoselected, so an endpoint that is simply unreachable is never chosen silently.

- No `--model`: the model last used with that provider, then a single-model endpoint's only entry, then an error naming the provider.

`--base-url` stays as an escape hatch for a local server, a corporate gateway or a test fixture; it overrides the URL, never the dialect.

Listed in autoselect order.

| `--provider` | Dialect | Key |
| --- | --- | --- |
| `openrouter` | openai-chat | `OPENROUTER_API_KEY` |
| `anthropic` | anthropic-messages | `ANTHROPIC_API_KEY` |
| `openai` | openai-responses | `OPENAI_API_KEY` |
| `ollama` | openai-chat | none |
| `llamacpp` | openai-chat | none |


## Build

```sh
make            # build
make check      # lint + test + budget; the full gate
make run        # one-shot against the mock provider
make repl       # interactive against the mock provider
make help       # every target
```

Plain `cargo build`, `cargo test` and `cargo clippy --all-targets -- -D warnings` work too; the Makefile only adds the budget gate and the smoke targets.

`scripts/test_openrouter.sh`, `test_openai.sh` and `test_anthropic.sh` run four scenarios against a real endpoint -- text, `read`, `bash`, then `write` with a read-back -- and exit non-zero if any of them misses. They take the key from the provider's usual environment variable or from `~/.config/minima/<provider>.key`, and work from a throwaway sandbox rather than the repo, because minima's tools have no path jail. The Anthropic one uses that vendor's OpenAI-compatibility endpoint; native `anthropic-messages` is the deferred amendment in [TODO.md](TODO.md).

Rust 1.88 or newer, for let-chains under edition 2024. `cargo test` also needs `python3`.

Unit tests live beside the module they exercise. `tests/live_path.rs` covers only what the in-process mock cannot reach -- the request body minima actually sends, and what it does against a gateway that serves no model list -- by driving the built binary against `tests/fixtures/fake_provider.py`. Behaviour that a unit test can already pin does not get a second assertion there.

Run without a network or an API key by replaying a scripted stream:

```sh
minima --mock mock/read-then-answer.json -p "what is this package?"
minima --mock mock/say-hi.json            # interactive
```

A mock script is a JSON array of turns, each an array of steps: `{"text": ...}`, `{"tool_call": {...}}`, `{"usage": {...}}`. One turn is consumed per provider round-trip.

Configuration, in precedence order: flags, then environment, then `$XDG_CONFIG_HOME/minima/`, which also holds the model cache, `history.txt` and `state.json`. minima creates the directory 0700 and the files it owns 0600, because prompts are written verbatim.

| Variable | Meaning |
| --- | --- |
| `MINIMA_PROVIDER` | registry id; unset, the first key variable set decides |
| `MINIMA_MODEL` | model id; unset, the one last used with it |
| `MINIMA_BASE_URL` | override the endpoint, never the dialect |
| `MINIMA_API_KEY` | overrides the provider's own key variable |
| `NO_COLOR` | any value turns colour off |
| `RUST_LOG` | `tracing` filter; `minima=debug` logs requests with auth redacted |

## Licence

MIT.
