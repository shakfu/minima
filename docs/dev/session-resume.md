# Session resume

How minima could save a conversation and continue it in a later run. Proposed 2026-10-04 against minima 0.6.0. Not implemented. Decisions marked **Open** need an answer before code.

## What a session is

`Agent::messages` (`src/agent.rs`) is the whole conversation: the system message, then user, assistant and tool messages. Compaction rewrites it in place. Every other `Agent` field is derived from config or recomputed:

- `tools`, `overhead`: from `config.dialect` and the system prompt.
- `used`, `pending`: token accounting. After a resume, set `used = 0` and `pending` to the estimate of every message. `check_context` and compaction then work unchanged.

So a session file is `messages`, plus enough metadata to choose and check the provider.

## File

```
{
  "schema": 1,
  "id": "20261004T153012-3f9a",
  "root": "/Users/sa/projects/x",
  "provider": "anthropic",
  "model": "claude-opus-5-5",
  "dialect": "anthropic-messages",
  "updated_at": 1791141012,
  "messages": [ ... ]
}
```

- `Message`, `ToolCall` and `Role` derive `Serialize` and `Deserialize`, with `Role` in lowercase. Alternative: a separate on-disk type. That would decouple the format from the internal type, at the cost of a conversion in both directions. The schema number already covers format changes, so derive.
- The system message is not stored. It is rebuilt on resume, so an edited `AGENTS.md` or skills list applies. The first resumed request misses the prompt cache either way.
- Location: `config_dir()/sessions/<root-hash>/<id>.json`. minima keeps its model cache and history in `config_dir()` today. `$XDG_STATE_HOME` would be the more correct place, but would add a second directory convention. **Open.**
- Directory 0700 and file 0600, through `config::restrict_to_owner`. Written with the tmp-plus-rename from `state.rs::store`.
- Another schema number is refused with an error naming the file. `state.rs` discards a stale file silently. That fits a preference, but a transcript is worth more.

## When it is written

- After each `Agent::run` returns, whether it ended with an answer, a cancel or an error.
- After a compaction, including `/compact`.

The whole file is rewritten each time. Alternative: append JSON lines. Appending is O(1) per turn, but compaction replaces the start of the conversation, so append mode still needs a full rewrite. Tool output is capped (`tools::cap`), so a file of a few MB costs milliseconds to rewrite.

A crash mid-turn loses that turn only. The previous write is still intact.

## Loading

1. Parse the file and check `schema`.
2. Rebuild the system message.
3. Give any assistant tool call without a result a `tools::CANCELLED` result. A run killed between the call and its result leaves one, and every dialect rejects an unanswered call.
4. If the dialect differs from the one saved, clear every `replay` and print a note. `replay` holds dialect-shaped reasoning and message items (`src/provider/mod.rs`), which another dialect cannot read.
5. The provider and model come from the file unless `-P` or `-m` is given.

Unverified: whether Anthropic accepts thinking blocks signed by a different model, and whether OpenAI accepts `encrypted_content` from a different model or key. If not, a model change within one dialect must also clear `replay`. Clearing it is safe in every case: the provider loses earlier reasoning, and the request still succeeds. The simplest rule is to clear `replay` on any provider or model change. **Open**. To settle it, extend `scripts/test_anthropic.sh` and `test_openai.sh`: save a tool-use turn, then resend it under another model. Deferred 2026-10-04.

## CLI

- `--continue`: resume the newest session for this root.
- `--resume ID`: resume a given session.

Both work with `-p`, so a script can chain prompts. The `--json` `result` record gains `session`, the id. No REPL command lists sessions; `ls` on the directory does that.

## Retention and privacy

A session holds every tool result. That includes any file `read` opened, so a `.env` can end up on disk. The history file already stores prompts, but tool output is a wider category.

- Redaction is not proposed. Pattern-matching secrets misses some and corrupts others.
- Keep the newest 20 sessions per root and delete older files on write. **Open:** count, age, or none.
- Saving by default or only on request is **Open**. Recommended: on by default, with `--no-session` and `MINIMA_NO_SESSION`. Claude Code and Codex both save by default; a resume that needs a flag set before the crash is rarely there when wanted.

## Cost

Estimated, not measured: 150-200 lines in a new `src/session.rs`, the derives, two flags, and the hook in `Agent::run`. Tests:

- round trip through the file
- an unpaired tool call gets a result
- `replay` is cleared on a dialect change
- another schema is refused
- the file mode is 0600

## Not proposed

- Branching or forking a session.
- Sharing a session across roots.
- Saving `used` and cost totals. Token usage is reported again on the first resumed turn. Cost totals restart, as they do for each run today.
