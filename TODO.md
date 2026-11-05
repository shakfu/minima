# TODO

## Critical

## High

- [ ] **`cargo fmt --check` fails; CI's lint job would fail on commit.** `src/frontend/repl.rs:564`, the `split_once(...).unwrap_or(...)` line from the uncommitted `/model` work. Run `cargo fmt`, then `make lint`. (REVIEW.md finding 1.)

## Medium

- [ ] **Session resume**. Design in `docs/dev/session-resume.md`; four decisions marked Open there. Note `src/cache.rs` has an XDG path, a schema version, an endpoint-keyed filename and a tmp-plus-rename atomic write, and `config::restrict_to_owner` already sets 0700/0600 for the history file. Persistence is open; this widens what is stored, not whether anything is. (Estimated 120-180 lines.)

- [ ] **The Linux half of the writable-set audit.** `docs/dev/root-sandbox.md` records that on macOS no cache entry is needed for a build to succeed, only for a fetch; go is an exception, remeasured 2026-10-04. The original claim that a denied `$CARGO_HOME` breaks the build was measured on Linux under Landlock and is untested against that question; CI runs `ubuntu-24.04`, so it can settle it. Trigger the `sandbox-linux` workflow and record the result in `root-sandbox.md`.

- [ ] **`/model <current id>` resets the window and drops `--context`.** `Agent::switch_model` sets `context` to 128k, and clears `max_output` and `pricing`, even when the model is unchanged. Make re-selecting the current model a no-op, or document it. Add a test. (REVIEW.md finding 2.)

- [ ] **`/models` and `/model` cannot be cancelled.** Both run `block_on` on the network with no `Cancel` check, so Esc and Ctrl-C do nothing for up to 10 s to connect or 300 s without data. Select the fetch against `cancel.cancelled()`, as `or_cancelled` does for tools. (REVIEW.md finding 3.)

- [ ] **Transport retries may re-send an accepted request.** `src/provider/http.rs:79` retries every `send()` failure except builder errors, including a read timeout after the server accepted the request. That bills twice. Retry only `is_connect()`, or document the cost in the README. (REVIEW.md finding 4; inferred, not run.)

- [ ] **`cargo audit` did not finish in review.** The advisory fetch and scan of 290 crates exceeded 300 s. Rerun on a connected machine before the next release and record the result. (REVIEW.md finding 7.)

## Low

- [ ] **Redact the API key and warn on plaintext `--base-url`.** `Config` derives `Debug` with `api_key: String` (`src/config.rs:184-190`), so a future `?config` log would leak it. Give `Config` a manual `Debug` that redacts the key. Warn when `--base-url` is `http://` and the host is not loopback. `--api-key` also appears in `ps`; the env route is the alternative to document. (REVIEW.md finding 5; inferred.)

- [ ] **`docs/dev/atomic-writes.md` line references are stale.** `sandbox_command` is cited at `bash.rs:186`; it is at line 195. Replace line numbers with function names, which do not drift. (REVIEW.md finding 6.)

- [ ] **Codemode: `js-only` on fix tasks.** Optional `js` was rejected 2026-10-24; see `docs/dev/codemode.md` ("Conclusion"). Only if `js-only` is worth pursuing as a product: compare it with `line` using `scripts/history_bench.py`.

- [ ] **Create-without-delete for the writable set.** Landlock has separate `MakeReg`, `WriteFile`, `RemoveFile` and `Truncate` bits and the ruleset grants `AccessFs::from_all`; SBPL has `file-write-create` and `file-write-data` apart from `file-write-unlink`. A package store the agent can add to but not delete from would match the accident model exactly. Risk is a half-written package with no way to clean it up. See `docs/dev/root-sandbox.md`.

- [ ] **Network denial on Linux.** Landlock ABI 4 (Linux 6.7) adds `AccessNet::BindTcp` and `ConnectTcp`. Denying both under `--sandbox` closes part of the exfiltration gap without bwrap. TCP only: UDP, and so DNS, stays open. Needs a floor above today's 6.2, or ABI detection that applies the rule where available. zerostack does this with a per-call network namespace (`sandbox-network = false`).

- [ ] **Credential masking.** zerostack hides `~/.ssh`, `~/.aws`, `~/.gnupg` and similar from sandboxed commands. Seatbelt can deny reads directly with `(deny file-read* (subpath ...))`. Landlock rules only grant access, so hiding `~/.ssh` means granting read on everything else, and reads are unbounded today.
