# Changelog

## Unreleased

### Fixed

- The context check counted only the token total the previous response reported. Tool results and a new prompt added since were not counted, so one turn of large outputs could send a request past the window. They are now estimated at 4 bytes a token until the next response reports a total. The estimate errs low, because refusing a request that would have fit costs more than sending one the provider rejects.

- A tool call streamed with an id or arguments but no name was dropped without notice. If it was the only call, the plan text was reported as the answer. It now fails the turn, and none of that turn's calls run. A fragment with no id, name or arguments is still ignored. The turn fails instead of running the calls that did arrive, because they may depend on the lost one.

- With `--json`, the `result` record's `text` held the last turn's text even when the run failed or was cancelled after a turn that called tools. That text was the model's plan, not an answer. `text` is now empty unless `outcome` is `complete`. The `turn` records still carry it.

### Changed

- The `--sandbox` kernel policy and the `write`/`edit` path check moved to the [`sanduk-sandbox`](https://github.com/shakfu/sanduk-rs) crate, so sanduk and pma share one implementation. Behaviour is unchanged, except that a protected-path refusal now reads `.git is not writable` without naming the tools. The policy tests moved with the code; the tests of minima's own notes stay here.

- Tool call lines read `[tool] <name> <detail>`, as in myra: `[tool] read src/lib.rs:1-400`, `[tool] bash cargo test`. The prefix marks them apart from the answer text; bash lost its `$ ` form so every tool follows one pattern.

## 0.5.0

### Added

- `--sandbox` bounds writes to the working directory -- or `--root DIR` -- and is off by default, leaving minima as unbounded as 0.4.0. On, `bash` and everything it starts run under a kernel policy: Landlock on Linux, Seatbelt on macOS, writes permitted under the root, `$TMPDIR`, `/dev/null` and the ecosystem caches. `write` and `edit` run in minima's own process, which the kernel policy does not reach, so they are held to the root by a path check. One confined command runs at startup, so an unsupported platform fails there rather than on the model's first tool call.

  A switch rather than the three modes this work first had, `--confine none|paths|fs`. The middle mode bounded `write` and `edit` but not `bash`, and read as confinement while every command still reached the whole filesystem. Off by default because Landlock needs kernel 6.2 and the preflight refuses to start below it; Debian 12, RHEL 9 and Ubuntu 22.04 GA sit under that floor. Falling back quietly instead would leave the user believing there is a boundary.

  Reads are never bounded. The network is open either way, so denying reads would hide headers, toolchains and dependency sources without closing exfiltration. The caches are writable so that fetching a dependency works: measured on macOS, compilation itself survives a denied cache, but `cargo add`, `go get` and `npm install` do not, and a lost cache costs a re-download rather than work.

- The writable caches are the entries a fetch or build writes, not their parents: `registry/`, `git/` and the lock and last-use files under `$CARGO_HOME`; `pkg/mod` and `pkg/sumdb` under the first `$GOPATH` entry, and `$GOMODCACHE`; `$XDG_CACHE_HOME` and `~/.npm`; and on macOS the per-user cache directory beside `$TMPDIR` and the `go-build`, `pip`, `com.apple.python`, `org.swift.swiftpm`, `ccache` and `deno` entries of `~/Library/Caches`. `$CARGO_HOME/bin` and `$GOPATH/bin` are on `PATH`, so a file written there would run unconfined in the next shell, and `~/Library/Caches` holds every app's cache. So `cargo install` and `go install` fail under `--sandbox`, and a relocated cache such as `$GOCACHE` needs `--writable`.

  Measured with the rest denied: cargo without its lock files warns and fetches unlocked; go without `pkg/sumdb` fails verifying every new module; `ccache` and `deno` fail without their entry; `swiftc` and `clang -fmodules` fail without the per-user cache directory; go, pip, python and SwiftPM run uncached.

- At startup `--sandbox` creates those cache entries under a `$CARGO_HOME` or `$GOPATH` that exists, and `$XDG_CACHE_HOME` when its parent does. A confined command cannot create them, so on a new machine the first fetch would fail; Landlock also drops a grant on a path that does not exist yet. Created by minima rather than documented as an unconfined first `cargo fetch`, which CI and new machines would miss.

- On macOS `--sandbox` also denies preference writes, `open`, and signals to processes outside the command's sandbox. None of them is a file write: `defaults write` hands the plist to cfprefsd, `open` has launchd start an app, one the command just built included, and `kill` reaches the user's other processes. A command still signals its own descendants. `open` is denied outright rather than behind a flag, because the agent rarely needs it and the user can run it; a denied `open` gets a note, since LaunchServices reports only `error -54`. macOS only: Landlock scopes signals from ABI 6, above the ABI 3 floor, and Linux has neither daemon.

- `--sandbox` sets `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` empty for `bash`. Under sccache a cacheable compile, which is every registry dependency, runs in the sccache server with the server's bounds: unconfined if it was started outside, so a proc macro could write anywhere, and a server a confined build starts keeps that policy after minima exits, failing every later cache hit on the machine. Empty rather than unset, because empty also overrides `build.rustc-wrapper` in cargo config. Confined builds lose sccache's cache.

- `.github/workflows/ci.yml` runs `make lint`, `make test` and `scripts/test_sandbox.py` on every push and pull request, on `ubuntu-24.04` and `macos-15`. The macOS runner is what makes the Seatbelt half of the sandbox a tested claim rather than an asserted one; lint runs there too, because clippy sees only the code compiled for the host. `.github/workflows/sandbox-macos.yml` runs `scripts/test_sandbox_macos.py` on demand, kept out of CI because it takes minutes and depends on package mirrors.

- A command that fails under `--sandbox` with text that looks like a denied write gets a note naming the policy and `--writable`. The kernel returns `EPERM` or `EACCES` and the program prints its own message, which never mentions minima, so a model reads `Operation not permitted` and retries the command or reaches for `sudo`. Matching the message is a heuristic: an ordinary permission error gets the note too, and a translated system gets nothing, which is cheaper than the retry loop it replaces.

  A command that fails with `sandbox_apply: Operation not permitted` gets a note naming the nested `sandbox-exec` and `swift build --disable-sandbox` instead. SwiftPM compiles a changed `Package.swift` under its own `sandbox-exec`, which Seatbelt refuses inside a sandbox, and `--writable` cannot fix that.

- `--writable DIR`, repeatable, adds a directory to the set `--sandbox` leaves writable for `bash`. The built-in set covers the caches a dependency fetch needs, and cannot cover every ecosystem: R, OCaml, Haskell and Stack keep installed packages in a user-level store, and enumerating them in minima would be a list that drifts. The flag is what makes an incomplete built-in set safe, so the person who knows they use `~/.opam` says so.

  Refused without `--sandbox`, where nothing is bounded for it to widen, rather than accepted as a no-op the user would read as a grant. It never widens `write` or `edit`: those stay inside the root, which keeps one sentence true of the file tools with no exceptions. Paths resolve at startup and a missing one fails there, because a typo that is dropped quietly leaves the user believing they granted access, and the sandbox preflight then runs against the real policy.

- The `--json` result record carries `sandbox` and `writable`, naming the bounds the run used. A program driving minima cannot see the flags it was started with, and whether `bash` was bounded changes what a failed tool call means. Reported once at the end rather than in a record of its own, since the result is already the summary a caller reads.

- Under `--sandbox`, `write` and `edit` refuse a path under the root whose resolved form has a `.git` component or one beginning `.env`. Losing an object store or a secret costs work that no later turn can rebuild, and the file tools have no reason to reach either. `.envrc` and the `.env` templates are exempt: they are committed, hold no secret, and are the file an agent edits when a feature adds a config variable.

  The check is resolved-path and component-wise, unlike the substring test the feature is modelled on, so `.github/`, `.gitignore` and `env.sample` are untouched and `../x/.env` is not. It is not a boundary: `bash` ignores it on both platforms. Landlock grants an access if any rule met while walking the path grants it, so no hole can be cut in the rule that grants the root, and enumerating the root's children instead would cost the ability to create a file at the top level of the project. Seatbelt does take a trailing deny, but a boundary that held only on macOS would be trusted on Linux.

- `scripts/test_sandbox.py` runs real commands under `--sandbox` and checks the disk rather than minima's report: a cargo build and a git commit that must succeed, and writes escaping the root by redirect, `cd ..`, symlink, Python, `rm`, truncation and a background job that must not. A mock plays the model, so it needs no key and runs unchanged on Linux and macOS. `SANDBOX=off` is the control, under which the escaping writes land. Under `--sandbox` a second session runs `cargo add`, `go mod tidy` and `uv venv` against an empty `CARGO_HOME` and `GOPATH` and a missing `XDG_CACHE_HOME`, and reports whether cargo kept its last-use record. It was chosen over an integration test in `tests/` because the cargo case makes it slow and it needs a kernel with Landlock.

- `scripts/test_sandbox_macos.py` runs real toolchains and daemon-mediated writes under `--sandbox` on macOS: dependency fetches through cargo, uv, pip, npm and go, swift and clang module builds, and `defaults`, `launchctl`, `security` and `kill` aimed outside the root. It also reports two gaps the policy cannot close: nested `sandbox-exec` is refused, so minima's own suite fails under `--sandbox`, and `launchctl disable` still marks a login agent disabled, a record that survives reboot. No SBPL rule tried blocks it; launchd refuses every other `launchctl` change from a sandboxed caller.

### Fixed

- A tool note in the REPL wraps instead of being cut at 80 columns. After a long stderr line the cut removed whatever the note appended, such as `--sandbox`'s hint on a denied write or the notice that background jobs are still running. A routine result is still cut to one line.

## 0.4.0

### Added

- `-m` for `--model` and `-P` for `--provider`. `-p` stays `--prompt`, matching `claude -p`.

- The REPL starts with `minima <version>` in bold cyan, or plain when colour is off. `-p` prints no banner, since its stdout is the answer.

- `/exit` leaves the REPL, as `/quit` does. Neither is recorded in `history.txt`, and older entries of either are dropped when it loads.

- The REPL prints one usage line per prompt: context used against the window, tokens in and out, and the cost in USD. `--json` carries it as `cost` in `turn` and `result` records, with `cost_estimated` true when minima computed it. OpenRouter reports the cost itself. For `openai` and `anthropic`, minima estimates it from OpenRouter's public price list and marks it `~`. It fetches that list from openrouter.ai without a key, at most once a day, even when OpenRouter is not the provider. The estimate prices cached input at the cache rate, which OpenAI bills at a tenth of the input rate, and applies long-prompt tiers. The price list was chosen over a table in minima, which would go stale. There is no estimate under `--base-url`, since a gateway need not bill at the vendor's rates.

### Changed

- The REPL shows each tool call on one line, by its target and result: `read src/lib.rs:495-994 -> 500 lines`, `$ cargo test -> exit 101: ...`. It showed the raw JSON arguments and a flattened preview of the output, and printed an unlabelled token count after every round-trip. That count was one request's prompt plus output, so it was neither context used nor spend. A failed call shows only the error's root cause, since the call line already names the target, and a blank line separates the answer from the tool lines. In `--json`, a failed call's `tool_result` carries that root cause as `note`, which was null.

- Autoselect tries the provider last used before the table order, while its key is set. With several keys set, a bare `minima` repeats the last provider and its model instead of switching to the first key in the table.

- The REPL is an inline ratatui viewport instead of a reedline prompt: an input box with a status bar below it, pinned to the bottom of the terminal. Output still goes to the terminal's own scrollback. The status bar shows the working directory, or a spinner and elapsed time during a turn, then the model, context used and the session's cost. The input box grows to 6 rows. Enter submits, Alt-Enter or Ctrl-J adds a newline, Up and Down or Ctrl-P and Ctrl-N browse history, and Ctrl-R searches it as readline does, except that Enter accepts the match into the input instead of submitting it. Typing continues during a turn and is kept for the next prompt. `history.txt` keeps reedline's format, so existing history carries over. The viewport was chosen over a scroll region or a permanent completion menu under reedline, since reedline clears everything below the prompt on each repaint. It costs 20 more crates. When the terminal gets narrower, ratatui clears the visible screen; the transcript stays in scrollback.

- `bash` no longer warns the user about stderr output when the command exits 0. cargo, git and pip write progress there, so a passing `cargo test` showed as `stderr: Updating crates.io index`. The model still gets the stderr line, which catches a failing stage in a pipeline that exits 0.

- The README and `docs/dev/root-sandbox.md` point to the experimental filesystem sandbox on the `sandbox` branch. This build has no sandbox.

- `--help` wraps at 80 columns, or the terminal width if narrower, and its flag descriptions are shorter. Wrapping needs clap's `wrap_help` feature, which adds `terminal_size`. Without it, clap does not wrap at all.

### Fixed

- An OpenAI model's context window comes from OpenRouter's model list when neither `--context` nor OpenAI's list gives one. OpenAI's `/models` reports no window, so minima assumed 128k and refused `gpt-5.6-luna` at about 12% of its 1.05M window. The listed window replaces only that fallback.

- A stream that ends without a terminal event is an error, not a finished turn. A proxy or a dropped connection can close an SSE stream after partial text or a complete-looking tool call, and minima took the text as the answer or ran the call. The Chat dialect now reports `finish_reason` as a terminal event of its own, separate from `[DONE]`: a server that omits the sentinel would otherwise fail every turn, and treating the stop reason as the end of the stream would stop the read before the usage frame that follows it.

- `bash` bounds what it captures while the command runs, keeping the first and last 16 KiB of each stream. The 32 KiB cap on a tool result was applied only after the tool returned, so a command that writes without stopping grew the buffer until the process died. `read` caps a single line the same way, and refuses anything but a regular file: `/dev/zero` has neither a newline nor an end.

- `write` and `edit` replace a file by renaming a temporary file beside it, carrying over the destination's mode and following a symlink to its target. Writing in place truncates first, so a kill -- including SIGTERM and SIGHUP, which exit through `process::exit` -- or a full disk could leave half a file where the user's only copy was.

## 0.3.0

### Added

- `AGENTS.md` in `$XDG_CONFIG_HOME/minima/`, then `AGENTS.md` in the working directory, are appended to the system prompt, following the [agents.md](https://agents.md) convention. Project instructions come last. Parent directories up to a repository root are not read, so the prompt matches the directory the environment section names.

- Skills in `$XDG_CONFIG_HOME/minima/skills/<name>/SKILL.md`, following the [Agent Skills](https://agentskills.io/specification) convention. The system prompt lists each skill's path and its YAML frontmatter as written; the model reads the file with `read` when a task needs it. Passing the frontmatter unparsed was chosen over a YAML dependency or a partial parser: the model reads YAML, and minima needs no field from it. A skill is skipped with a warning if its frontmatter is missing, has no `description` line, or exceeds 4096 bytes.

### Changed

- `bash` refuses a command that starts a login shell, such as `bash -lc '...'`, and tells the model to pass the inner command. GPT models wrap commands this way despite the tool description, in 2 of 7 live runs of `gpt-5-nano`. On macOS the login profile runs `path_helper`, which put `/usr/bin/python3` ahead of Homebrew's, so a wrapped command could run different programs. Refusing was chosen over stripping the wrapper, which needs shell-quote parsing and would run a command other than the one logged. A plain `bash -c` wrapper still runs. A command that needs a login profile must source it, as in `. ~/.bash_profile && <command>`.

## 0.2.2

### Added

- Releases also publish a macOS aarch64 binary. It is dynamically linked, since macOS supports no static binaries.

### Fixed

- The 0.2.1 release published no binaries. Its x86_64 check looked for "statically linked", but Rust links x86_64 musl as static-pie, which `file` reports as "static-pie linked".

## 0.2.1

### Added

- `--json`, with `-p`, prints one JSON record per line on stdout: `turn`, `tool_call`, `tool_result`, `retry`, then a final `result` with the outcome, answer, error, turn count and token totals. Plain `-p` gives a caller only the exit code, because errors go to stderr and nothing marks the end of the answer. An error is written to the `result` record and not repeated on stderr.

- Tagged releases publish static musl binaries for Linux x86_64 and aarch64, with a `.sha256` for each. Built on native runners because cross-compiling `aws-lc-sys` needs a cross C toolchain.

### Fixed

- The Anthropic model list authenticates with `x-api-key`, as completions do, and follows the `after_id` cursor. minima sent a bearer token, which Anthropic accepts but a gateway checking `x-api-key` refuses, and it read only the first page of 20. The list's `max_input_tokens` now sets the context window; without it every Anthropic model fell back to 128,000 tokens, so a 1M-token model was refused at 128k.

## 0.2.0

### Changed

- A cancelled `-p` run exits 130 instead of 0, so a script can tell it from a finished run.

- `--api-key` and `MINIMA_API_KEY` now require `--provider`. A key does not name its vendor, so autoselect could send an OpenAI key to `openrouter.ai`. Requiring the flag was chosen over ignoring the key under autoselect, which would silently drop a key the user set.

- `bash` runs `bash -c` instead of `bash -lc`. Login files added startup time and printed into tool results; commands now inherit minima's environment, including `PATH`.

- The `bash` tool description tells the model not to wrap commands in another shell. OpenAI models wrapped them in `bash -lc` in live runs.

- `bash` runs each command in its own process group. A timeout or cancel kills the group, not only `bash`, and output produced before a timeout is kept.

- Background jobs outlive their call but not minima. The call result says they are still running, and minima kills them when it exits, including on SIGHUP (exit 129) and SIGTERM (exit 143). This was chosen over killing them when the call ends, which would stop a server started in one call from serving the next.

- The system prompt names `bash` as the command shell instead of `$SHELL`, which on macOS is zsh.

- The model is remembered per provider only after its first turn streams. A mistyped `--model` is no longer reused by later runs.

- Requests time out: 10 s to connect, 300 s between reads. A stalled stream no longer hangs `-p`. The read timeout bounds silence, not stream length.

- A full context window is refused before sending. The turn that fills it is kept, and its tool calls are answered but not run.

- Ctrl-C cancels a REPL turn, as Esc does. Cancelling while a request is pending, or during retry backoff, takes effect at once.

- A provider asking to retry after more than 60 s is reported instead of waited out.

- History excludes only `/quit`. Prompts starting with a path, such as `/etc/hosts`, are recorded.

- `read` streams the file, so memory follows the lines returned, not the file size. A multi-line `edit` matches CRLF files.

- The README states that there is no approval gate, and no longer cites a line target.

### Fixed

- A cancel between tool calls left later calls without results. Every dialect rejects such a transcript, so each following request in the REPL session failed until restart. Every call now gets a result, including calls skipped for a cancel, a full window, or the output limit.

- A cancelled tool call still sent the next request, which was paid for and then dropped.

- A cancel could lose to already-buffered stream events, keeping part of the cancelled turn.

- A `bash` command that started a background job waited for the full timeout. The job held the stdout pipe open, so EOF never arrived. Reading now stops 100 ms after `bash` exits.

- `edit` with an empty `old` rewrote the file: an empty pattern matches between every character. It is now refused.

- A response cut off at the output token limit looked like malformed tool arguments, and the model retried until `--max-turns`. All three dialects now report the stop. Cut-off calls are not run, their arguments are stored as `{}`, and cut-off text is an error.

- Assistant text and error bodies reached the terminal with escape sequences intact. Text from a file the model read could retitle the terminal or write to the clipboard.

- Chat Completions: calls without an `index` merged into one; mid-stream `error` chunks were dropped; an empty assistant turn was sent without `content`.

- Tool output with stdout lacking a trailing newline ran into the first stderr line.

## 0.1.0 - 2026-09-16

Initial release.
