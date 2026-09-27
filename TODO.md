# TODO

## Critical

## High

## Medium

- [x] **Cancel `read`, `write` and `edit`; refuse FIFOs before open.** Only `bash` receives `cancel` (`src/tools/mod.rs:110-125`), and `src/agent.rs:161` awaits the call without a `select!`. `read` opens before checking `meta.is_file()` (`src/tools/read.rs:31-42`), so a FIFO with no writer blocks forever; Esc, Ctrl-C and later SIGINTs do nothing. Wrap the dispatch in `src/agent.rs:160-163` in a `select!` on `cancel.cancelled()` answering `tools::CANCELLED`, and call `tokio::fs::metadata` before the open. Both are needed: the `select!` alone leaves the blocking-pool thread stuck. (REVIEW.md #1, confirmed.)

- [x] **Create the `atomic` temp file 0600.** `write_then_rename` creates it with `File::create` (0666 minus umask), writes and syncs, then copies the target's mode (`src/tools/atomic.rs:27-45`). A 0600 target's new contents are world-readable for the duration, `fsync` included. Open with `OpenOptions::mode(0o600)`, keep the chmod before rename. `cache.rs`, `state.rs`, `history.rs` share the order but write inside a 0700 directory. (REVIEW.md #2, confirmed.)

- [x] **Raise or derive Anthropic `max_tokens`.** `MAX_TOKENS` is 8192 (`src/provider/anthropic.rs:15`). A `write` over about 8000 tokens stops at `max_tokens`; `write` replaces the whole file, so a split needs `bash` appends, which bypass the `.git`/`.env` protection. Raise the constant or take the output limit from the model list. Chat and Responses send no limit. (REVIEW.md #3, traced.)

- [x] **Retry transport errors, or correct the README.** `Error::is_retryable` matches `RateLimited` and `Server` only (`src/provider/mod.rs:148`); a failed `send()` maps to `Error::Other` (`src/provider/http.rs:79`). A TCP reset or connect timeout ends a `-p` run on the first attempt, yet `README.md:90` claims "up to 4 connection retries". Classify `reqwest::Error::is_connect()` and `is_timeout()` before the body starts as retryable. (REVIEW.md #4, traced.)

- [ ] **Kernel-enforced `write` and `edit` under `--sandbox`.** `confine_path` checks a path that `atomic::replace` reopens by name in minima's unconfined process, so a symlink swapped in by a background `bash` job moves the write outside the root. Run the replacement in a sandboxed child via a hidden `__replace` subcommand. See `docs/dev/atomic-writes.md`. Deferred 2026-09-27: first decide whether `--sandbox` covers a hostile model or only mistakes; if only mistakes, drop threat model 2 from `root-sandbox.md` instead.

- [ ] **`message_delta` usage is a correction, not a total.** Handled in `turn.rs::merge_usage`, but the rule is a convention rather than something the wire states. A provider reporting only a total and no halves falls back to the reported figure.

- [ ] **Session resume**. Note `src/cache.rs` has an XDG path, a schema version, an endpoint-keyed filename and a tmp-plus-rename atomic write, and `config::restrict_to_owner` already sets 0700/0600 for the history file. Persistence is open; this widens what is stored, not whether anything is. (Estimated 120-180 lines.)

- [x] **Extended thinking**. `provider/anthropic.rs` parses `thinking_delta` and `signature_delta` and drops both, and minima never sends a `thinking` block to ask for them. Responses has the parallel feature: hax requests `reasoning.encrypted_content` and replays it, because with `store:false` that is the only way a chain of thought survives the tool calls of one turn (`responses_body.c:118-140`).

- [x] **Prompt cache markers**. `prompt_cache_key` is sent on both OpenAI dialects and confirmed accepted. Anthropic is different: caching is explicit `cache_control` markers on the last system block, the last tool and the last message, not a heuristic (`anthropic_body.c:193-225`). minima sends none, so every Anthropic turn reprocesses the whole transcript.

- [x] **Streaming markdown**. Every live run shows it: models emit `**3 .txt files**` and minima prints the asterisks. hax spends 1,764 lines on `render/markdown.c` and `markdown_table.c`. Streaming crates do exist (`mdstream`, `streamdown-parser`, `mdriver`), but none fit: the REPL already commits finished lines, so `pulldown-cmark` renders each one. Tables are not aligned.

- [ ] **The Linux half of the writable-set audit.** `docs/dev/root-sandbox.md` records that on macOS no cache entry is needed for a build to succeed, only for a fetch. The original claim that a denied `$CARGO_HOME` breaks the build was measured on Linux under Landlock and is untested against that question; CI runs `ubuntu-24.04`, so it can settle it.

## Low

- [x] **Always sanitise invalid tool arguments.** `{}` replaces arguments only when the turn was truncated (`src/agent.rs:119-125`). A complete turn with invalid JSON keeps the raw string, and `chat.rs:38` and `responses.rs:34` resend it every request; `anthropic.rs:44` already substitutes. A gateway that parses Chat arguments may then fail every later request (untested against OpenRouter). Drop the `if truncated` guard. Also: the error `tool arguments were not valid: {raw}` bypasses `cap` (`src/tools/mod.rs:139`), so a failed 30 KiB `write` enters the context twice. (REVIEW.md #5, inferred.)

- [x] **`Retry-After` HTTP-date form.** Parsed as integer seconds only (`src/provider/http.rs:104-109`); the date form falls back to exponential backoff. (REVIEW.md #4.)

- [x] **Pin `sanduk-sandbox`.** `README.md:12` gives `cargo install minima`, which ignores `Cargo.lock` and takes the newest 0.1.x. A 0.1.x release then changes what `--sandbox` enforces without a minima release or CI run. Pin `sanduk-sandbox = "=0.1.0"`, or document `cargo install --locked`. (REVIEW.md #7, traced.)

- [x] **Rename-based writes break hard links and drop ownership.** The rename replaces the inode: after `ln h1 h2` and a `write` to `h1`, `h2` keeps the old contents. Owner, ACLs and xattrs are not copied; only the mode is. Document it in `docs/dev/atomic-writes.md` and the README, or write in place when `nlink > 1`. In `src/tools/atomic.rs`: a dangling symlink fails `canonicalize`, so the rename replaces the link with a regular file (lines 53-61); and the parent directory is not synced after the rename. (REVIEW.md #6, confirmed.)

- [x] **Count the first request in the context check.** `check_context` adds the prompt estimate to `used`, which is 0 before the first response (`src/agent.rs:89-91`). The system prompt, `AGENTS.md` (unbounded, `src/prompt.rs:64`), the skills list and tool schemas are not estimated. Cost: one rejected request. (REVIEW.md #8, traced.)

- [x] **Stop refetching the OpenRouter price list on every failed start.** `price::lookup` refreshes a stale cache (`src/price.rs:113`) and stores nothing on failure. Behind a firewall that drops packets, each `openai` or `anthropic` start waits the 10 s connect timeout; only `--base-url` disables the lookup (`src/price.rs:104`). Store `fetched_at` on failure, or add a flag. (REVIEW.md #9, traced.)

- [x] **Documentation drift.** (REVIEW.md #10.)
  - `docs/dev/atomic-writes.md:7,23` cites `src/tools/mod.rs:138` and `src/tools/bash.rs:413,481`; `confine_path` moved to `sanduk-sandbox`, `sandbox_command` is at `bash.rs:192`.
  - `README.md:148` omits the `"cut"` mock step (`src/provider/mock.rs:13`).
  - `README.md:60` is one ~300-word paragraph; make it a list like the rest of the section.

- [x] **CI hardening.** (REVIEW.md #11.)
  - Pin `dtolnay/rust-toolchain` and `Swatinem/rust-cache` by commit; `release.yml` builds published binaries with them.
  - Run `cargo audit` or an equivalent in a workflow.
  - Verify `upload-artifact@v4` with `download-artifact@v7` in `release.yml`.

- [ ] **Create-without-delete for the writable set.** Landlock has separate `MakeReg`, `WriteFile`, `RemoveFile` and `Truncate` bits and the ruleset grants `AccessFs::from_all`; SBPL has `file-write-create` and `file-write-data` apart from `file-write-unlink`. A package store the agent can add to but not delete from would match the accident model exactly. Risk is a half-written package with no way to clean it up. See `docs/dev/root-sandbox.md`.

- [x] **Declare Unix-only.** Two `cfg(not(unix))` arms let a non-Unix build compile and break three documented properties: `signal_group` returns `false` without signalling (`src/tools/bash.rs:61`), so a timeout or a cancel kills nothing and background jobs outlive minima; `restrict_to_owner` is a no-op (`src/config.rs:343`), so the config directory is not 0700/0600; and `exit_on_hangup_or_terminate` is not registered (`src/main.rs:58`). `const SIGKILL: i32 = 9` (`src/tools/bash.rs:58`) is already dead -- the non-Unix `signal_group` discards its `signal` argument. Put `compile_error!` under `cfg(not(unix))` in `main.rs` and delete both arms: a build failure naming the reason beats a binary that runs and violates the README. The Windows build is inferred from the cfg structure, not verified; the target is not installed here.
