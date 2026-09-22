# minima

A minimal coding agent harness with a tiny feature set.

`minima` tests how small a usable agent harness can be when the ecosystem carries its capabilities. It was inspired by Oleksandr Chekhovskyi's [hax](https://github.com/OleksandrChekhovskyi/hax).

**IMPORTANT** minima runs every tool call **without asking**. It **has no approval gate**. By default, `--confine paths` bounds `write` and `edit` to the working directory -- or `--root DIR` -- and refuses the protected paths under it. `bash` is not bounded in that mode and reaches the whole filesystem; `--confine fs` adds a kernel policy that bounds it. Reads are never bounded and neither is the network, so an adversarial model can still read and send whatever the user can. Confinement stops accidents outside the project; it does not contain an untrusted prompt. Run minima in a container for that.

## Install

```sh
cargo install minima
```

Requires Rust 1.88 or newer. To build from a checkout, see [Build](#build).

`--confine fs` needs Linux 6.2 or newer, or macOS. Below that floor it refuses to start; the default `--confine paths` has no such requirement. See [Features](#features) for what each mode bounds.

```sh
% minima -h
A minimal coding agent for the terminal

Usage: minima [OPTIONS]

Options:
      --root <DIR>      Directory the agent works in, and the bound --confine
                        applies. Default: the current directory
      --confine <MODE>  How much a tool call is bounded [default: paths]
                        [possible values: none, paths, fs]
      --writable <DIR>  Another directory bash may write to under --confine fs.
                        Repeatable
  -p, --prompt <TEXT>   Answer one prompt, print the result, exit
      --json            With -p: print JSON lines, ending in a `result` record
  -P, --provider <ID>   Provider id. Default: the first one whose key is set
                        [env: MINIMA_PROVIDER=]
  -m, --model <ID>      Model id. Default: the last one used with this provider
                        [env: MINIMA_MODEL=]
      --base-url <URL>  Override the provider's endpoint. The wire format stays
                        the same [env: MINIMA_BASE_URL=]
      --api-key <KEY>   Provider key. Requires --provider [env: MINIMA_API_KEY]
      --no-color        Disable colour. Also off for non-terminals or when
                        NO_COLOR is set
      --context <N>     Context window in tokens. Default: the cached value for
                        the model [env: MINIMA_CONTEXT=]
      --mock <PATH>     Replay a scripted JSON stream instead of the network
      --max-turns <N>   Max provider round-trips per user turn [default: 32]
      --refresh-models  Re-fetch the model list, ignoring the cache
  -h, --help            Print help (see more with '--help')
  -V, --version         Print version
```

## Features

- **Providers:** 5 in a fixed registry, over 3 wire formats: openai-chat, openai-responses and anthropic-messages. See [Providers](#providers).

- **Tools:** 4. `read` returns numbered lines, 2000 by default. `write` creates or replaces a file. `edit` replaces one exact string. `bash` runs a command under `bash -c`.

- **Shell commands:** each call runs in its own process group. A timeout (120 s default, 600 s cap) or a cancel kills the group. Background jobs outlive the call and die with minima. A login-shell wrapper such as `bash -lc` is refused, because a login profile can reorder `PATH`.

- **Confinement:** `--confine` takes one of three modes. `none` bounds nothing. `paths`, the default, bounds `write` and `edit` by the root and refuses the protected paths under it, leaving `bash` unbounded. `fs` adds the platform's filesystem sandbox to `bash` and its descendants, which may then write only under the root, `$TMPDIR`, `/dev/null` and the ecosystem caches (`registry/`, `git/` and the lock files under `$CARGO_HOME`, `$XDG_CACHE_HOME`, `pkg/mod` and `pkg/sumdb` under `$GOPATH` or `$GOMODCACHE`, `~/.npm`, and on macOS the per-user cache directory and the `go-build`, `pip`, `com.apple.python`, `org.swift.swiftpm`, `ccache` and `deno` entries of `~/Library/Caches`, not the directory, which every app shares). On macOS it also denies preference writes, signals to processes outside the command's sandbox, and `open`, which would start an app outside it; so `open page.html` and `open https://...` fail too. At startup minima creates those cache directories and cargo's lock files, under a `$CARGO_HOME` or `$GOPATH` that exists, since a confined command cannot. `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` are set empty, because a wrapper such as sccache runs the compile in a server outside the command's bounds. So `fs` is not `paths` applied to `bash`: it is a wider set for `bash`, because no build works without the caches, and the same narrow set for the file tools. Reads are unrestricted in every mode. Under `fs` one confined command runs at startup, so a platform that cannot install the sandbox fails there rather than mid-turn, and a command whose output looks like a denied write gets a note naming the policy and `--writable`, shown to both the model and the user, since `Operation not permitted` on its own tells neither of them anything.

  | Platform | Mechanism | Requires |
  |-|-|-|
  | Linux | Landlock | kernel 6.2 (ABI 3) |
  | macOS | Seatbelt, via `sandbox-exec` | -- |
  | other | none | a mode below `fs` |

  Debian 12, RHEL 9 and Ubuntu 22.04 on its 5.15 GA kernel sit below the Linux floor, so `--confine fs` refuses to start there. Ubuntu's `linux-generic-hwe-22.04` clears it. This is why `fs` is not the default: a default that refuses to start on the common server distributions is worse than one the user opts into.

- **Protected paths:** `write` and `edit` refuse any path under the root with a `.git` component, or one beginning `.env`, matched on the resolved path. `.envrc` and the templates -- `.env.example`, `.env.sample`, `.env.template` -- are committed files and stay writable. `bash` is not bound by this and neither is `read`. It stops a misdirected `write`, not a command that means to remove the file.

- **Writable paths:** `--writable DIR`, repeatable, adds a directory to what `bash` may write under `--confine fs`. For a toolchain whose store sits outside the project: `~/.opam`, `~/.stack`, an R library. It never widens `write` or `edit`, which stay inside the root in every mode, and it is refused under `none` and `paths`, where nothing bounds `bash` for it to widen. Paths are resolved at startup, and one that does not exist is an error rather than a silent skip.

- **Unsafe mode:** `--confine none` keeps the selected working directory and bounds nothing. It prints a warning at startup.

- **Modes:** an interactive REPL, and headless `-p`, printing text or JSON lines with `--json`. `/exit`, `/quit` or Ctrl-D on an empty input leaves the REPL.

- **REPL:** an input box with a status bar below it, pinned to the bottom of the terminal; output scrolls above it into the terminal's scrollback. Enter submits, Alt-Enter or Ctrl-J adds a newline, Up and Down or Ctrl-P and Ctrl-N browse history, Ctrl-R searches it, and Ctrl-C clears the input. Typing continues during a turn.

- **JSON output:** one record per line: `turn`, `tool_call`, `tool_result`, `retry`, then a final `result`. `turn` and `result` carry token counts, `cost` in USD or null, and `cost_estimated`; `result` also names the bounds the run used, as `confine` and `writable`.

- **Display:** one line per tool call, such as `read src/lib.rs:1-400 -> 400 lines` or `$ cargo test -> exit 101: ...`. A routine result is cut to fit; a note or an error wraps onto further rows, so a hint at its end is never lost. After each prompt, one line gives context used, tokens in and out, and the cost. OpenRouter reports the cost; for OpenAI and Anthropic it is estimated from OpenRouter's public price list and marked `~`. The status bar shows the working directory, or a spinner and elapsed time during a turn, then the model, context used and the session's cost.

- **Cancellation:** Esc or Ctrl-C cancels a REPL turn, including a pending request or a retry wait. A cancelled `-p` run exits 130.

- **Instructions:** `AGENTS.md` in the config directory, then `AGENTS.md` in the working directory, are appended to the system prompt.

- **Skills:** `skills/<name>/SKILL.md` in the config directory. The system prompt lists each skill's path and frontmatter; the model reads the file when a task matches.

- **Context:** the window comes from `--context`, the provider's model list, or OpenRouter's list for an OpenAI model, whose own list gives none. Once the last turn's token count nears the window, the next request is refused before sending. There is no compaction.

- **Network:** up to 4 connection retries with backoff. Requests time out after 10 s to connect or 300 s without data. With `openai` or `anthropic`, minima also fetches OpenRouter's public model list, without a key and at most once a day, for prices and missing context windows. `--base-url` turns this off.

- **Persistence:** a model list cache, prompt history, the last provider, and the last model per provider. See [Build](#build) for where they live.

- **Colour:** on for a terminal, off for a pipe, `--no-color` or `NO_COLOR`.

- **Offline runs:** `--mock` replays a scripted JSON stream instead of calling the network.

### Providers

Each registry entry fixes a base URL, a dialect and a key variable, so `--provider` and `--model` are the whole selection, and both have a fallback:

- No `--provider`: the provider last used, if its key variable is still set, then the first entry below whose key variable is set. So a bare `minima` repeats the last provider and model. Local servers carry no key and are never autoselected, so an endpoint that is simply unreachable is never chosen silently.

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

`scripts/test_sandbox.py` runs real commands under `--confine fs` -- a cargo build, a git commit, and writes that escape the root by redirect, `cd ..`, symlink, Python, `rm`, truncation and a background job -- then checks the disk rather than minima's report. A mock plays the model, so it needs no key. `CONFINE=paths` runs it as a control, where the escaping `bash` writes should land.

`scripts/test_sandbox_macos.py` runs real toolchains under `--confine fs` on macOS -- dependency fetches through cargo, uv, pip, npm and go, and swift and clang module builds -- with writes a daemon makes for the command (`defaults`, `launchctl`, `security`) and `kill` aimed outside the root. It needs the network. minima's own `cargo test` cannot run under `--confine fs` on macOS, because Seatbelt refuses a nested `sandbox-exec`. For the same reason `swift build` needs `--disable-sandbox` there: SwiftPM compiles a changed `Package.swift` under its own `sandbox-exec`.

Rust 1.88 or newer, for let-chains under edition 2024. `cargo test` also needs `python3`.

Unit tests live beside the module they exercise. `tests/live_path.rs` covers only what the in-process mock cannot reach -- the request body minima actually sends, and what it does against a gateway that serves no model list -- by driving the built binary against `tests/fixtures/fake_provider.py`. Behaviour that a unit test can already pin does not get a second assertion there. `tests/headless.rs` checks `-p` exit codes and that background jobs die with minima.

Run without a network or an API key by replaying a scripted stream:

```sh
minima --mock mock/read-then-answer.json -p "what is this package?"
minima --mock mock/say-hi.json            # interactive
```

A mock script is a JSON array of turns, each an array of steps: `{"text": ...}`, `{"tool_call": {...}}`, `{"usage": {...}}` with token counts and an optional `cost`, and `"truncated"` for a response cut off at the output token limit. One turn is consumed per provider round-trip.

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
