# TODO

## Critical

## High

## Medium

- [ ] **`message_delta` usage is a correction, not a total.** Handled in `turn.rs::merge_usage`, but the rule is a convention rather than something the wire states. A provider reporting only a total and no halves falls back to the reported figure.

- [ ] **Session resume**. Note `src/cache.rs` has an XDG path, a schema version, an endpoint-keyed filename and a tmp-plus-rename atomic write, and `config::restrict_to_owner` already sets 0700/0600 for the history file. Persistence is open; this widens what is stored, not whether anything is. (Estimated 120-180 lines.)

- [ ] **Extended thinking**. `provider/anthropic.rs` parses `thinking_delta` and `signature_delta` and drops both, and minima never sends a `thinking` block to ask for them. Responses has the parallel feature: hax requests `reasoning.encrypted_content` and replays it, because with `store:false` that is the only way a chain of thought survives the tool calls of one turn (`responses_body.c:118-140`).

- [ ] **Prompt cache markers**. `prompt_cache_key` is sent on both OpenAI dialects and confirmed accepted. Anthropic is different: caching is explicit `cache_control` markers on the last system block, the last tool and the last message, not a heuristic (`anthropic_body.c:193-225`). minima sends none, so every Anthropic turn reprocesses the whole transcript.

- [ ] **Streaming markdown**. Every live run shows it: models emit `**3 .txt files**` and minima prints the asterisks. hax spends 1,764 lines on `render/markdown.c` and `markdown_table.c`. No crate does incremental streaming render -- `termimad` renders finished documents.

## Low
