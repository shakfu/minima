# TODO

## Critical

## High

## Medium

- [ ] **`message_delta` usage is a correction, not a total.** Handled in `turn.rs::merge_usage`, but the rule is a convention rather than something the wire states. A provider reporting only a total and no halves falls back to the reported figure.

- [ ] **Session resume**. Note `src/cache.rs` has an XDG path, a schema version, an endpoint-keyed filename and a tmp-plus-rename atomic write, and `config::restrict_to_owner` already sets 0700/0600 for the history file. Persistence is open; this widens what is stored, not whether anything is. (Estimated 120-180 lines.)

- [ ] **Extended thinking**. `provider/anthropic.rs` parses `thinking_delta` and `signature_delta` and drops both, and minima never sends a `thinking` block to ask for them. Responses has the parallel feature: hax requests `reasoning.encrypted_content` and replays it, because with `store:false` that is the only way a chain of thought survives the tool calls of one turn (`responses_body.c:118-140`).

- [ ] **Prompt cache markers**. `prompt_cache_key` is sent on both OpenAI dialects and confirmed accepted. Anthropic is different: caching is explicit `cache_control` markers on the last system block, the last tool and the last message, not a heuristic (`anthropic_body.c:193-225`). minima sends none, so every Anthropic turn reprocesses the whole transcript.

- [ ] **Streaming markdown**. Every live run shows it: models emit `**3 .txt files**` and minima prints the asterisks. hax spends 1,764 lines on `render/markdown.c` and `markdown_table.c`. No crate does incremental streaming render -- `termimad` renders finished documents.

- [ ] **The Linux half of the writable-set audit.** `docs/dev/root-sandbox.md` records that on macOS no cache entry is needed for a build to succeed, only for a fetch. The original claim that a denied `$CARGO_HOME` breaks the build was measured on Linux under Landlock and is untested against that question; CI runs `ubuntu-24.04`, so it can settle it.

## Low

- [ ] **Create-without-delete for the writable set.** Landlock has separate `MakeReg`, `WriteFile`, `RemoveFile` and `Truncate` bits and the ruleset grants `AccessFs::from_all`; SBPL has `file-write-create` and `file-write-data` apart from `file-write-unlink`. A package store the agent can add to but not delete from would match the accident model exactly. Risk is a half-written package with no way to clean it up. See `docs/dev/root-sandbox.md`.

- [ ] **Declare Unix-only.** Two `cfg(not(unix))` arms let a non-Unix build compile and break three documented properties: `signal_group` returns `false` without signalling (`src/tools/bash.rs:61`), so a timeout or a cancel kills nothing and background jobs outlive minima; `restrict_to_owner` is a no-op (`src/config.rs:343`), so the config directory is not 0700/0600; and `exit_on_hangup_or_terminate` is not registered (`src/main.rs:58`). `const SIGKILL: i32 = 9` (`src/tools/bash.rs:58`) is already dead -- the non-Unix `signal_group` discards its `signal` argument. Put `compile_error!` under `cfg(not(unix))` in `main.rs` and delete both arms: a build failure naming the reason beats a binary that runs and violates the README. The Windows build is inferred from the cfg structure, not verified; the target is not installed here.
