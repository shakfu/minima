# Changelog

## Unreleased

### Added

- `--confine none|paths|fs` bounds what a tool call can change. The default `paths` bounds `write` and `edit` by the working directory -- or `--root DIR` -- in minima's own process. `fs` adds a kernel policy to `bash` and everything it starts: Landlock on Linux, Seatbelt on macOS, writes permitted under the root, `$TMPDIR`, `/dev/null` and the ecosystem caches, and one confined command at startup so an unsupported platform fails there rather than on the model's first tool call.

  `fs` is not the default because Landlock needs kernel 6.2 and the preflight refuses to start below it. Debian 12, RHEL 9 and Ubuntu 22.04 GA all sit under that floor, and a default that refuses to start on the common server distributions is worse than one the user asks for. Falling back quietly instead was the other option and is worse still: the user then believes there is a boundary. One ordered flag rather than a `--sandbox` and a `--path-protection` switch, so the two bounds sit on one axis and neither implies anything about the other.

  Reads are not bounded in any mode. The network is open either way, so denying reads would hide headers, toolchains and dependency sources without closing exfiltration. The caches are writable so that fetching a dependency works: measured on macOS, compilation itself survives a denied cache, but `cargo add`, `go get` and `npm install` do not, and a lost cache costs a re-download rather than work. `~/Library/Caches` is the macOS half of the `$XDG_CACHE_HOME` entry, where Go keeps its build cache.

- `.github/workflows/ci.yml` runs `make lint` and `make test` on every push and pull request, on `ubuntu-24.04` and `macos-15`. The macOS runner is what makes the Seatbelt half of the sandbox a tested claim rather than an asserted one.

- A command that fails under `--confine fs` with text that looks like a denied write gets a note naming the policy and `--writable`. The kernel returns `EPERM` or `EACCES` and the program prints its own message, which never mentions minima, so a model reads `Operation not permitted` and retries the command or reaches for `sudo`. Matching the message is a heuristic: an ordinary permission error gets the note too, and a translated system gets nothing, which is cheaper than the retry loop it replaces.

- `--writable DIR`, repeatable, adds a directory to the set `--confine fs` leaves writable for `bash`. The built-in set covers the caches a dependency fetch needs, and cannot cover every ecosystem: R, OCaml, Haskell and Stack keep installed packages in a user-level store, and enumerating them in minima would be a list that drifts. The flag is what makes an incomplete built-in set safe, so the person who knows they use `~/.opam` says so.

  Refused under `none` and `paths`, where nothing bounds `bash` for it to widen, rather than accepted as a no-op the user would read as a grant. It never widens `write` or `edit`: those stay inside the root in every mode, which keeps one sentence true of the file tools with no exceptions. Paths resolve at startup and a missing one fails there, because a typo that is dropped quietly leaves the user believing they granted access, and the sandbox preflight then runs against the real policy.

- The `--json` result record carries `confine` and `writable`, naming the bounds the run used. A program driving minima cannot see the flags it was started with, and whether `bash` was bounded changes what a failed tool call means. Reported once at the end rather than in a record of its own, since the result is already the summary a caller reads.

- `write` and `edit` refuse a path under the root whose resolved form has a `.git` component or one beginning `.env`. Losing an object store or a secret costs work that no later turn can rebuild, and the file tools have no reason to reach either. `.envrc` and the `.env` templates are exempt: they are committed, hold no secret, and are the file an agent edits when a feature adds a config variable.

  The check is resolved-path and component-wise, unlike the substring test the feature is modelled on, so `.github/`, `.gitignore` and `env.sample` are untouched and `../x/.env` is not. It is not a boundary: `bash` ignores it on both platforms. Landlock grants an access if any rule met while walking the path grants it, so no hole can be cut in the rule that grants the root, and enumerating the root's children instead would cost the ability to create a file at the top level of the project. Seatbelt does take a trailing deny, but a boundary that held only on macOS would be trusted on Linux.

### Fixed

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
