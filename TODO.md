# Candidate amendments

Options, not a roadmap. Each one costs lines against the 4,000 in README.md, and nothing here is committed to. Record the decision when one is taken or dropped.

## Taken

- 2026-09-16: `anthropic-messages` and `openai-responses`, alongside `openai-chat`. The translation layer this needed is `src/provider/mod.rs`; each dialect owns its own module.

- 2026-09-16: `--provider` over a fixed registry, with `--base-url` as an endpoint-only override.

- 2026-09-16: colour by default, off for a pipe, `--no-color` and `NO_COLOR` honoured.

- 2026-09-16: budget raised 3,000 to 4,000 to pay for the three dialects.

- 2026-09-16: provider autoselect from the environment, and the model last used with a provider remembered in `state.json`. The registry's order is the autoselect order and is tested as a contract.

## Known gaps

Defects and half-measures, not scope choices. Small enough to fix when they bite.

- **Anthropic model listing.** `src/cache.rs` sends bearer auth and parses a flat `data` array. The Messages dialect authenticates with `x-api-key` plus `anthropic-version`, and Anthropic's `/v1/models` is paged with `after_id` cursors (`anthropic_models.c:117-190` in hax). A keyless probe cannot tell whether a valid bearer is accepted there, so the auth half is unconfirmed; the paging half is certain and means only the first page is ever seen. It degrades rather than breaks: `--model` given, the fetch is skipped; `--model` omitted against Anthropic, it fails with the endpoint's own message. Fix is to pass the dialect into `Models::refresh` and follow cursors, roughly 40 lines.

- **Config resolution had a disk side effect.** `resolve()` used to write `state.json`, which made its own unit tests write to the developer's real `~/.config/minima`. The write moved to `main.rs`; `tests/live_path.rs` now sets `XDG_CONFIG_HOME` per fixture. Worth remembering as a shape: a resolver that persists is a resolver whose tests persist.

- **`message_delta` usage is a correction, not a total.** Handled in `turn.rs::merge_usage`, but the rule is a convention rather than something the wire states. A provider reporting only a total and no halves falls back to the reported figure.

## Session resume

Status: deferred, not started, 2026-09-16.

The strongest pull on the freeze, and cheap, because the fiddly parts already exist: `src/cache.rs` has an XDG path, a schema version, an endpoint-keyed filename and a tmp-plus-rename atomic write, and `config::restrict_to_owner` already sets 0700/0600 for the history file. Persistence is open; this widens what is stored, not whether anything is.

Estimated 120-180 lines.

### Open decisions

- What a session is addressed by. `--continue` for the most recent and `--resume <id>` for a named one keeps it to two flags. A picker is in the README's absent list, so resume stays flag-driven or the absent list changes too.

- What gets written. The full `Vec<Message>` is the whole context, tool results included, so a session file is roughly the size of the conversation. Decide whether tool results are stored verbatim or re-truncated on load.

- Secrets. Tool output lands on disk verbatim: `bash` output can contain keys, tokens and `env` dumps. Reuse `config::restrict_to_owner` for both the directory and the files.

- Pruning. Without a cap the directory grows without bound. A count or age limit is a few lines; an interactive pruner is not, and is absent by default.

- Schema drift. Reuse `cache.rs`'s version field and drop unreadable sessions rather than migrating them.

- The session id is also the natural `prompt_cache_key`, which `provider/http.rs` currently derives from the pid and the clock.

### What it displaces

The README scope table's `Persistence` row changes again. Update it in the same commit.

## Extended thinking

Status: not started.

`provider/anthropic.rs` parses `thinking_delta` and `signature_delta` and drops both, and minima never sends a `thinking` block to ask for them. Responses has the parallel feature: hax requests `reasoning.encrypted_content` and replays it, because with `store:false` that is the only way a chain of thought survives the tool calls of one turn (`responses_body.c:118-140`).

Both need the same thing minima does not have: a message variant that carries opaque provider state back unchanged. Anthropic requires thinking blocks and their signatures to be replayed (`anthropic_events.c:177`); OpenAI binds encrypted reasoning to the model that produced it, so replaying it after a model switch is rejected (`responses_body.c:47-54`).

Estimated 150-250 lines for both, most of it the provenance rule rather than the parsing.

## Prompt cache markers

Status: not started.

`prompt_cache_key` is sent on both OpenAI dialects and confirmed accepted. Anthropic is different: caching is explicit `cache_control` markers on the last system block, the last tool and the last message, not a heuristic (`anthropic_body.c:193-225`). minima sends none, so every Anthropic turn reprocesses the whole transcript.

Roughly 40 lines, and the one item here that pays for itself in latency and cost rather than capability.

## Streaming markdown

Status: not started, and deliberately absent from the README.

Every live run shows it: models emit `**3 .txt files**` and minima prints the asterisks. hax spends 1,764 lines on `render/markdown.c` and `markdown_table.c`. No crate does incremental streaming render -- `termimad` renders finished documents.

The cheap subset is fenced code blocks and inline code only, roughly 300 lines, which is most of what is actually lost. Bold, headings and tables are worth less than they cost.
