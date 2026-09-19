# minima

A minimal coding agent harness with a tiny feature set.

`minima` tests how small a usable agent harness can be when the ecosystem carries its capabilities. It was inspired by Oleksandr Chekhovskyi's [hax](https://github.com/OleksandrChekhovskyi/hax).

**IMPORTANT** minima runs every tool call **without asking**. It **has no approval gate**. The default filesystem sandbox confines the model to the current directory; pass `--root DIR` to select another project. Runtime files such as system binaries remain readable so commands can execute. Network access is not restricted. `--no-sandbox` restores unrestricted tool access and should be used only with trusted prompts.

## Install

```sh
cargo install minima
```

Requires Rust 1.88 or newer. To build from a checkout, see [Build](#build).

```sh
% minima --help
A minimal coding agent for the terminal

Usage: minima [OPTIONS]

Options:
      --root <DIR>       Directory in which the agent operates; defaults to the current directory
      --no-sandbox       Disable the filesystem sandbox and root path checks
  -p, --prompt <TEXT>   Headless: answer this prompt, print the result, exit
      --json            With -p: print one JSON record per line on stdout, ending in a `result` record
      --provider <ID>   Which provider to talk to: fixes the endpoint, the wire format and the key variable. Left out, minima takes the first provider whose key variable is set [env: MINIMA_PROVIDER=]
      --model <ID>      Left out, minima reuses the model last used with this provider [env: MINIMA_MODEL=]
      --base-url <URL>  Override the provider's endpoint, for a local server or a gateway. Never changes the wire format: a different shape is a different provider, not a different address [env: MINIMA_BASE_URL=]
      --api-key <KEY>   Overrides the provider's key variable. Requires --provider [env: MINIMA_API_KEY]
      --no-color        Print without colour. Colour is off anyway when stdout is not a terminal, or when NO_COLOR is set
      --context <N>     Context window in tokens. Falls back to the cached value for the model [env: MINIMA_CONTEXT=]
      --mock <PATH>     Replay a scripted JSON stream instead of calling the network
      --max-turns <N>   Refuse to keep going after this many provider round-trips in one user turn [default: 32]
      --refresh-models  Re-fetch the model list even if the cache is fresh
  -h, --help            Print help
  -V, --version         Print version
```

## Features

- **Providers:** 5 in a fixed registry, over 3 wire formats: openai-chat, openai-responses and anthropic-messages. See [Providers](#providers).

- **Tools:** 4. `read` returns numbered lines, 2000 by default. `write` creates or replaces a file. `edit` replaces one exact string. `bash` runs a command under `bash -c`.

- **Shell commands:** each call runs in its own process group. A timeout (120 s default, 600 s cap) or a cancel kills the group. Background jobs outlive the call and die with minima. A login-shell wrapper such as `bash -lc` is refused, because a login profile can reorder `PATH`.

- **Root boundary:** `--root DIR` changes to `DIR`, rejects file-tool paths outside it, and confines `bash` with Landlock on Linux or Seatbelt on macOS. It fails rather than running unconfined when the platform sandbox cannot be installed.

- **Unsafe mode:** `--no-sandbox` keeps the selected working directory but disables filesystem confinement and root path checks.

- **Modes:** an interactive REPL with history, and headless `-p`, printing text or JSON lines with `--json`.

- **Cancellation:** Esc or Ctrl-C cancels a REPL turn, including a pending request or a retry wait. A cancelled `-p` run exits 130.

- **Instructions:** `AGENTS.md` in the config directory, then `AGENTS.md` in the working directory, are appended to the system prompt.

- **Skills:** `skills/<name>/SKILL.md` in the config directory. The system prompt lists each skill's path and frontmatter; the model reads the file when a task matches.

- **Context:** the window comes from the provider's model list or `--context`. Once the last turn's token count nears the window, the next request is refused before sending. There is no compaction.

- **Network:** up to 4 connection retries with backoff. Requests time out after 10 s to connect or 300 s without data.

- **Persistence:** a model list cache, prompt history, and the last model per provider. See [Build](#build) for where they live.

- **Colour:** on for a terminal, off for a pipe, `--no-color` or `NO_COLOR`.

- **Offline runs:** `--mock` replays a scripted JSON stream instead of calling the network.

### Providers

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
make check      # lint + test; the full gate
make run        # one-shot against the mock provider
make repl       # interactive against the mock provider
make publish    # cargo publish, verified in its own target dir
make help       # every target
```

Plain `cargo build`, `cargo test` and `cargo clippy --all-targets -- -D warnings` work too.

`scripts/test_openrouter.sh`, `test_openai.sh` and `test_anthropic.sh` run five scenarios against a real endpoint -- text, `read`, `bash`, `write` with a read-back, then a skill the prompt does not name -- and exit non-zero if any of them misses. They take the key from the provider's usual environment variable or from `~/.config/minima/<provider>.key`, and work from a throwaway directory rather than the repo. `XDG_CONFIG_HOME` points at a throwaway directory too, so your own `AGENTS.md` and skills stay out of the prompt. The Anthropic one speaks native `anthropic-messages`.

Rust 1.88 or newer, for let-chains under edition 2024. `cargo test` also needs `python3`.

Unit tests live beside the module they exercise. `tests/live_path.rs` covers only what the in-process mock cannot reach -- the request body minima actually sends, and what it does against a gateway that serves no model list -- by driving the built binary against `tests/fixtures/fake_provider.py`. Behaviour that a unit test can already pin does not get a second assertion there. `tests/headless.rs` checks `-p` exit codes and that background jobs die with minima.

Run without a network or an API key by replaying a scripted stream:

```sh
minima --mock mock/read-then-answer.json -p "what is this package?"
minima --mock mock/say-hi.json            # interactive
```

A mock script is a JSON array of turns, each an array of steps: `{"text": ...}`, `{"tool_call": {...}}`, `{"usage": {...}}`, and `"truncated"` for a response cut off at the output token limit. One turn is consumed per provider round-trip.

Configuration, in precedence order: flags, then environment, then `$XDG_CONFIG_HOME/minima/`, which also holds the model cache, `history.txt`, `state.json`, and optionally `AGENTS.md` and `skills/`. minima creates the directory 0700 and the files it owns 0600, because prompts are written verbatim.

| Variable | Meaning |
| --- | --- |
| `MINIMA_PROVIDER` | registry id; unset, the first key variable set decides |
| `MINIMA_MODEL` | model id; unset, the one last used with it |
| `MINIMA_BASE_URL` | override the endpoint, never the dialect |
| `MINIMA_API_KEY` | overrides the provider's own key variable; requires a named provider |
| `MINIMA_CONTEXT` | context window in tokens; unset, the cached value for the model |
| `NO_COLOR` | any value turns colour off |
| `RUST_LOG` | `tracing` filter; `minima=debug` logs requests with auth redacted |

## Licence

MIT.
