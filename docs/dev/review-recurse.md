# Review: `--recurse` (f21b122)

Reviewed 2026-11-19: `src/family.rs`, the hooks in `src/agent.rs` and `src/main.rs`, `tests/recursion.rs` and `docs/dev/recursion.md`. `make test` passes: 231 unit tests and 22 integration tests, 0 failures. Line numbers refer to that commit; `recursion.md` has since been rewritten.

## Bugs

### 1. `minima -r shutdown` run by an agent also kills that agent

`src/family.rs:538-545`

- `shutdown` skips only `std::process::id()`. When an agent calls the subcommand from `bash`, that is the subcommand's own pid, not the agent's.
- The system prompt says `shutdown` "stops every agent in the family." A root REPL model may run it to clean up its children. That sends SIGTERM to the root and closes the user's REPL.
- Fix: skip the member whose id is `MINIMA_SELF`. Or give the command a `--children` scope and point the prompt at that.

### 2. Mail is lost when `run` fails before the prompt is pushed

`src/agent.rs:204-225`

- `with_mail` empties the mailbox first. Four exits come before `messages.push`: the overhead `bail!`, a cancelled `compact_if_full`, a compaction error, and `check_context`.
- Any of them throws the mail away, including a child's final answer.
- Adding the mail can itself trigger `check_context`: a large child answer pushes a normal prompt over the window, and both are lost.
- Fix: read the mail without truncating it, and truncate only after the push.

### 3. A task that starts with `-` makes the child exit silently

`src/family.rs:818`

- Confirmed with the binary: `minima -p "-fix the tests"` fails with `error: unexpected argument '-f' found`.
- The child dies in clap, before `answer_parent` runs. The parent gets no mail. The only trace is in `<id>.log`.
- Fix: pass `format!("--prompt={task}")`.

### 4. The doc says a child's session id equals its member id. It does not.

`docs/dev/recursion.md:340`

- The member id comes from `new_id()` inside the short-lived `minima -r spawn` process, using that process's pid (`src/family.rs:787`).
- The session id comes from `Session::new`, using the child's own pid (`src/session.rs:65`). The timestamps can also differ.
- So `minima --resume <member id>` does not find the child's transcript.
- Fix: pass `MINIMA_ID` into `Session::new`, or remove the claim from the doc.

## Design questions

- **A child that hits the budget stops the root.** `shutdown_others` signals every other member, so a child's request can SIGTERM the user's interactive REPL. Alternative: stop only the subtree of the member that hit the limit, and let the root decide.
- **The check and the spend are not atomic.** N members can each pass `check_budget` and then spend. The overshoot is then bounded by N x `max_output`, not one `max_output`. The 5% reserve covers this only while N is small. The doc's "soft" caveat does not mention this.
- **The budget path blocks the async runtime.** `shutdown_others` runs inside the async `one_turn` and sleeps there: at least 100 ms, up to 5 s grace. It repeats on every later refused request.
- **The pid check depends on the binary's name.** `is_minima` matches `file_name == "minima"`. A renamed or symlinked install skips every member, and `shutdown` just reports "skipped" for each.

## Minor issues

- CHANGELOG says "a prompt that starts with their names is unaffected." That is false for the positional REPL prompt: `minima -r spawn a reviewer` now runs as a subcommand.
- `budget * (100 - RESERVE_PERCENT)` (`src/family.rs:428`) overflows when `--budget-tokens` is above about 1.9e17.
- In `docs/dev/recursion.md:357`, the "Cost:" line is filed under "Not done".
- Prompt injection: a child's text can contain a fake `[message from root (...)]` line inside the user message. This fits the accident model, but `docs/dev/recursion.md:245` says senders are tagged, which suggests more protection than there is.

## Test gaps

`tests/recursion.rs` covers spawn/recv, the root crossing the budget, and `agents` with no family. Not covered:

- mail drained into the next prompt (`with_mail`)
- `send`
- the `shutdown` subcommand
- a child crossing the budget
- sandbox inheritance

The sandboxed case is the one with the most code behind it: the family directory joins the writable set, the child runs with `--no-session`, and Seatbelt profiles nest.

## Open questions

- Is `shutdown` meant to kill the caller? This decides the fix for bug 1.
- Is a budget crossing meant to stop the root? This decides the first design question.

## Outcome (2026-11-19)

Implemented; `recursion.md` describes the result. `make test`: 240 unit tests and 29 integration tests, 0 failures.

### Fixes

- Bug 1: `shutdown` run by an agent stops only its descendants.
- Bug 2: gone with the mailbox.
- Bug 3: fixed.
- Bug 4: fixed; a child saves its session under its member id.
- Orphans: removed. A child stops when its parent exits.
- Budget: `--budget-tokens` is required.
- Mail as user input: the mailbox is replaced by answer files and `wait`.
- A child crossing the budget: stops only its own subtree.
- The check and the spend: atomic, through holds in the ledger.
- `stop_descendants` blocking the runtime: it runs on tokio's blocking pool.
- `is_minima`: compares with this executable's name; reads `/proc/<pid>/exe` on Linux.
- Spawn names: validated.
- The budget overflow: fixed.
- The CHANGELOG claim about prompts after `-r`: dropped; the entry is cut to 150 words.

### Found by the new tests and live runs

- **`shutdown` did nothing under the macOS sandbox**, which refuses `ps` and cross-sandbox signals. It now asks through a marker file; see `recursion.md`, Stopping.
- **Worst-case holds starved concurrent requests.** A hold of about 129,000 tokens let only two requests be in flight under a 300,000-token budget, and a child's first request failed. Requests now wait for held room and are granted less output on a short budget.
- **Chat and Responses sent no output cap**, so no grant was enforced on four of five providers. Inside a family, Chat now sends `max_tokens` and Responses `max_output_tokens`.
- **Cost holds on OpenRouter were $0.** The provider reports cost, so `pricing` stayed unset and nothing projected a hold's cost. Holds now project from `Config::rates`, loaded for OpenRouter's own provider too.

### Live runs

`scripts/test_recurse.py`, sandbox on. Each run's prompt asks the root to delegate two line counts and return the sum, 42.

| # | Model | Budget | Code | Checks | Spent | Root turns |
|-|-|-|-|-|-|-|
| 1 | `claude-haiku-5-5` | 300,000 | before grants and waiting | 13 of 14 | 15,468 tokens, $0.0013 | 4 |
| 2 | `claude-haiku-5-5` | 300,000 | grants and waiting | 13 of 13 | 12,606 tokens, $0.0010 | 3 |
| 3 | `openai/gpt-6-luna`, OpenRouter | 300,000 | output caps on Chat | 13 of 13 | 5,975 tokens, $0.0007 | 3 |
| 4 | `openai/gpt-6-luna`, OpenRouter | 300,000 | cost from `rates` | 13 of 13 | 5,920 tokens, $0.0007 | 3 |
| 5 | `claude-haiku-5-5` | 60,000 | as 4 | 13 of 13 | 12,635 tokens, $0.0010 | 3 |

1. A child's first request failed: the root's and a sibling's holds of about 129,000 each left 24,438 under the line. The root spawned a replacement and still answered 42, so three children were spawned, which failed the check for two.
2. At the same point, the child was granted the remaining 24,442 tokens, about 22,600 output. Peak held: 283,146 of the 285,000 line.
3. On Chat, a child was granted 26,380 and sent the matching `max_tokens`; OpenRouter accepted it. Every hold cost $0.
4. A full hold now costs $0.06409 against $0.00013 spent; the reduced hold $0.01286.
5. Every hold was the whole remainder, `57,000 - spent`. Both children waited for room and neither failed; requests ran one at a time, about 3 s of 7. The ledger is in `recursion.md`, Worked example.

On both models the root spawned, waited for each child before answering, and never ran `shutdown`. gpt-6-luna waited by id, Haiku by name.

### Open

- **Grants take the whole remainder.** Under a budget below one model ceiling, requests run one at a time (run 5). An equal share of the remainder would let them overlap, at the risk of cutting off a long response. Left as it is.
- **Cost holds project the worst case.** A full hold reserves about 500 times what a request spends (run 4), so a `--budget-cost` must cover the requests in flight, not their expected spend.
- **Cap enforcement is unverified.** No response in any run came near its cap. OpenRouter accepts `max_tokens`; Ollama and llama.cpp are untested.
- **A cancel costs its full hold.**
- **The Linux sandbox tests run only in CI.**
