# Review: `--recurse` (f21b122)

Reviewed 2026-11-19: `src/family.rs`, the hooks in `src/agent.rs` and `src/main.rs`, `tests/recursion.rs` and `docs/dev/recursion.md`. `make test` passes: 231 unit tests and 22 integration tests, 0 failures.

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
