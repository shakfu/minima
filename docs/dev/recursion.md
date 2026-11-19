# Recursive agents

`-r`/`--recurse` lets an agent start child agents and collect their answers: local fork/join, in `src/family.rs`. Off by default; without `-r`, nothing below runs. Live conversation between agents and people is out of scope: that is minos (`~/projects/personal/minos`), which minima may join later through a stdin stream-json mode.

How the design got here is in [History](#history).

## Model

- A **family** is a root agent and every agent it spawns, transitively.
- A **member** is one `minima` process in it. A child is a headless `minima -p` run: one task, its own context, its own process group.
- The agent reaches the family through `bash`, as it reaches `git`. No fifth tool.
- No daemon. Each member reads and writes the family's files itself, under `flock`.
- A child's final answer reaches its parent as the output of `minima -r wait`, never inside a prompt.
- A child stops when its parent exits. No member outlives the agent that started it.
- The root fixes a token budget for the whole family. Every request reserves against it first.

## Starting a family

```text
minima -r --budget-tokens N [--budget-cost USD] [OPTIONS] [-p PROMPT | PROMPT...]
```

- `--budget-tokens` is required on the root: it is the one bound on what the family can spend. The check is in `family::join`, not clap, because `spawn` starts children with `-r` and no budget.
- `--budget-cost` adds a second line in USD.
- A child given either flag refuses to start. It reads the budget from the registry.
- `--resume` with `-r` founds a new family. Nothing links a saved session to the family it ran in.

## Subcommands

Recognised only directly after `-r`, so `minima spawn the tests` stays a prompt.

| Command | Does |
|-|-|
| `minima -r spawn --name NAME [-P ID] [-m ID] -- TASK` | Starts a child on TASK and prints its member id. |
| `minima -r wait NAME_OR_ID [--timeout SECS]` | Prints the child's final answer, waiting for it. Default 100 s. |
| `minima -r agents [--family ID] [--prune]` | Lists the family, or drops ended members. |
| `minima -r shutdown [--family ID] [--grace SECS]` | Stops members. Default grace 5 s. |

- **`spawn`**
  - `NAME` is 1 to 32 of `[A-Za-z0-9_-]`, unique among the caller's running children. The `agents` table prints it, and the model reads that table.
  - The provider and model default to the caller's, from the registry. `--base-url`, `--api-key` and `--context` are not passed on; environment variables pass through as the process environment does.
  - `--mock PATH` exists for tests.
- **`wait`**
  - Resolves by member id, else by the name of a running member, else by a unique name.
  - Fails, naming the child's log, when the child exited without an answer.
  - Fails when the child is still running at the timeout. The default sits under `bash`'s 120 s timeout; the model runs `wait` again.
  - Reads the answer without removing it, so a second `wait` sees the same answer.
- **`agents`** prints `name role status model id`. `role` is `self`, `parent`, `child`, `sibling` or `-`.
- **`shutdown`**
  - Run by an agent: stops that agent's descendants, not the agent or its ancestors.
  - Run from outside any family: stops the whole family.
- **Which family** `agents` and `shutdown` act on: `--family`, else `$MINIMA_FAMILY`, else the newest family whose root is the current directory.

## The system prompt

`Family::prompt` appends one block to the system prompt. It names the agent and its parent, gives the four commands with the absolute path of the running binary, and says:

- run `wait` again while a child is still working;
- children stop when you exit, so wait for every answer before giving your own;
- children share your root, sandbox and budget.

Measured 2026-11-19 with `scripts/test_recurse.py` on `claude-haiku-5-5`: the root spawned both children, waited for each, gave the right sum, and never ran `shutdown`. The two runs used 15,468 and 12,606 tokens.

## Files

One directory per family: `<state>/family/<family id>/`. `<state>` is `$XDG_STATE_HOME/minima`, else `~/.local/state/minima`. The directory is 0700 and every file in it 0600.

| File | Holds |
|-|-|
| `registry.json` | What the root fixed (root, sandbox, writable set, budget) and every member. |
| `ledger.jsonl` | One line per hold or settlement; see [Budget](#budget). |
| `<id>.answer` | A child's final answer. |
| `<id>.log` | A child's stdout and stderr. |
| `<id>.stop` | Written by `shutdown`; the member exits when it sees it. |
| `lock` | Held with `flock` around every read-modify-write. |

A member is `{id, name, parent, pid, provider, model, started_at, ended_at}`. It is **running** while `ended_at` is unset and its pid exists.

Ids are `<unix seconds>-<pid>`, the shape of a session id:

- The family id is the root's member id.
- `spawn` mints a child's id before the child starts.
- A child saves its session under its member id, so `minima --resume <id>` opens its transcript.

Identity travels in the environment, everything else in the registry:

- `MINIMA_FAMILY` and `MINIMA_SELF` are set on every member, so its `bash` commands know whose they are.
- `MINIMA_PARENT`, `MINIMA_ID` and `MINIMA_NAME` are set on a starting child alone. It reads and removes them, so a `minima` its own `bash` starts does not take them for itself.

## A child's life

1. **Start.** `spawn` runs `minima --recurse --no-color --prompt=TASK` from `current_exe()`:
   - in the family's root, in its own process group, stdin null, output to `<id>.log`;
   - with `--no-session` under `--sandbox`, since the session directory is outside the writable set.

   The task is one `--prompt=` argument, so a task starting with `-` is not read as a flag.
2. **Join.** The child records its pid and reads the budget and its parent's pid from the registry. It applies the registry's root, sandbox and writable set verbatim, so it cannot widen them.
3. **Run.** One headless turn, budgeted like any other member.
4. **Answer.** It writes `<id>.answer`:
   - its last non-empty assistant text, else `(no answer)`;
   - `failed: <error>` on an error, or `cancelled`.

   It writes the answer after saving its session: the parent may exit as soon as it reads the answer, and the child with it.
5. **Exit.** It marks itself ended.

### Stopping

Every member runs one watch task, which polls once a second:

| Condition | Exit code | stderr |
|-|-|-|
| its own `<id>.stop` exists | 143 | `stopped by the family` |
| its parent's pid is gone | 1 | `the parent agent exited; stopping` |

SIGTERM and SIGHUP take the same path. It kills the member's `bash` jobs, which sit in their own process groups, marks the member ended, and exits. Measured with mock models (`tests/recursion.rs`): a child running `sleep 30` stops, with the `sleep`, within about a second of its parent's exit.

The watch is on the logical parent, the pid in the registry. The child's OS parent is the `spawn` subcommand, which exits at once, so `PR_SET_PDEATHSIG` would fire at once. Sampling once a second is safe from pid reuse: a pid is reused only once the pid counter wraps.

`shutdown`:

1. Writes `<id>.stop` for each target.
2. Waits up to the grace period for the targets to exit.
3. Sends SIGKILL to a survivor that `is_minima` confirms, and fails if one survives that.
4. Marks the targets ended and prunes.

`is_minima` compares the process's executable name with this binary's: `/proc/<pid>/exe` on Linux, `ps -o comm=` on macOS. `comm` is not read on Linux because Linux cuts it to 15 bytes.

`shutdown` asks through a marker rather than signalling first, because of the macOS sandbox. Measured 2026-11-19:

- Inside it, `/bin/ps` fails with `Operation not permitted`.
- A signal to a process outside the caller's sandbox fails with `EPERM`, per the profile's `(allow signal (target same-sandbox))`. A child started by an earlier `bash` call is outside the sandbox of a later one.
- `kill(pid, 0)` still tells a live process (`EPERM`) from a gone one (`ESRCH`), so `alive()` and the parent watch work.

### Pruning

Pruning drops an ended member only once its parent has ended too, so a running parent can still `wait` for the answer or read the log. It removes the member's answer, log and marker. A family with no members left is removed whole, ledger included.

`agents --prune` and every `shutdown` prune.

## Budget

Each provider request reserves before it is sent, and settles after.

### Reserve

`Agent::reserve` computes:

| | |
|-|-|
| input | what the window holds: `context - room` |
| ceiling | `min(max_output, room)`, with 32,000 where the model list gives no `max_output` |
| floor | 4,096 |

`Family::try_reserve` then reads the ledger and the registry, under the lock:

- **spent:** settled lines, plus the open holds of members no longer running. Such a request spent an unknown amount, at most its hold.
- **held:** the open holds of running members.
- **line:** the budget less a 5% reserve.
- **grant:** `min(ceiling, line - spent - held - input)`, lowered further to fit `--budget-cost` at the listed rates.

Then:

| Outcome | When | The agent |
|-|-|-|
| Granted | grant >= floor | Appends a hold of `input + grant` and sends the request. |
| Busy | only `held` stands in the way | Polls again every 250 ms. Esc cancels. |
| Fails | `spent` alone leaves less than `input + floor` | Stops its descendants, then fails the request and each later one. |

The grant caps `room`, and `room` sets the request's output cap: `max_tokens` on Messages and Chat, `max_output_tokens` on Responses. So the provider enforces the grant, and the budget can be spent to its end. A response that needs more than its grant is cut off at the output limit.

Outside a family, Chat and Responses send no cap: a cap there would impose minima's 32,000-token default where the model list gives no ceiling, and cut off responses the provider would finish. `Config::cap_output`, set when an agent joins a family, turns it on. On reasoning models (OpenAI Responses, and reasoning models through OpenRouter), reasoning counts against the cap, so a grant near the floor can end incomplete.

### Settle

- A completed request appends a line with its real usage. That line names the hold's line number and replaces it in every total.
- A request that does not complete (cancelled, failed, killed mid-stream) settles at its full hold. Without that, a running member's own open hold would keep it Busy.
- Retries inside one request share its hold. A response dropped mid-stream before a retry is not recorded.
- A hold's cost is projected from `Config::rates`, the rates OpenRouter lists for the model: its own listing for the `openrouter` provider, its mirror for `openai` and `anthropic`. A settled line's cost is the provider's report, else an estimate from the same rates. `rates` is kept apart from `pricing` because `pricing` also marks a cost as estimated, and OpenRouter's is reported. With no listed rates (local servers, a custom `MINIMA_BASE_URL`), a hold costs $0, so `--budget-cost` checks only what is spent. Measured 2026-11-19 on `openai/gpt-6-luna` through OpenRouter, before `rates`: every hold cost $0.

A ledger line:

```json
{"member":"1795078894-27006","at":1795078894,"tokens":24442,"cost":0.0,"hold":true}
{"member":"1795078894-27006","at":1795078895,"tokens":1657,"cost":0.0,"settles":4}
```

### Crossing

The member whose request fails stops its own descendants, not the family. Its parent:

- gets `failed: family token budget ...` from `wait`;
- is refused by its own next check, since the budget is the family's.

So the family stops top-down, and no member signals its parent. An interactive root stays open, and the user sees why.

### Worked example

A hold reserves one request. It is not a member's share of the budget: no member has one. Every member draws from the family's budget one request at a time:

1. **Reserve.** Before sending, write a hold of `input + grant`.
2. **Send.** The grant goes out as the request's output cap.
3. **Settle.** When the response completes, a line with the real usage replaces the hold.

`BUDGET=60000 scripts/test_recurse.py`, 2026-11-19, `claude-haiku-5-5`. The root's first request:

| | tokens |
|-|-|
| line: 60,000 less the 5% reserve | 57,000 |
| spent, held | 0, 0 |
| input: the system prompt and the prompt | about 1,800 |
| grant: `min(128,000, 57,000 - 0 - 0 - 1,800)` | about 55,200 |
| **hold:** `input + grant` | **57,000** |

The response used 1,858 tokens, and its settle line replaced the 57,000. The whole run's ledger:

| t | line | member | entry | tokens |
|-|-|-|-|-|
| 197 | 0 | root | hold | 57,000 |
| 198 | 1 | root | settles 0 | 1,858 |
| 198 | 2 | root | hold | 55,142 |
| 200 | 3 | root | settles 2 | 2,051 |
| 200 | 4 | count-a | hold | 53,091 |
| 201 | 5 | count-a | settles 4 | 1,658 |
| 201 | 6 | count-a | hold | 51,433 |
| 201 | 7 | count-a | settles 6 | 1,674 |
| 201 | 8 | count-b | hold | 49,759 |
| 203 | 9 | count-b | settles 8 | 1,659 |
| 203 | 10 | count-b | hold | 48,100 |
| 203 | 11 | count-b | settles 10 | 1,670 |
| 203 | 12 | root | hold | 46,430 |
| 204 | 13 | root | settles 12 | 2,065 |

- Each hold is `57,000 - spent`: the whole remainder, since the remainder is below the 128,000 ceiling.
- Both children were spawned at t=198. count-a's first hold came at t=200, once the root's settled; count-b's at t=201, once count-a's had. Each waited, Busy; neither failed.
- So under a budget below one ceiling, requests run one at a time. That costs time, about 3 s of 7 here, not spend. A grant limited to an equal share of the remainder would let them overlap, at the risk of cutting off a response that needs more than its share.

### Measured

2026-11-19, `claude-haiku-5-5` (`max_output: 128000`), 300,000-token budget:

- Each request held about 129,000 tokens and used about 2,000.
- With the root's and one child's holds open, the second child was granted 24,442 tokens: about 22,600 output after its input.
- The peak held was 283,146 tokens, under the 285,000 line.
- Before the grant and the wait existed, the same run failed that child's first request.

## Sandbox

- The parent's `bash` policy confines every process started from it, a child included. A child's own `--sandbox` flag adds nothing to what it may write; setting it from the registry gives the child `--no-session` and the sandbox's error notes.
- Under `--sandbox`, the family directory joins the writable set: the subcommands run under the sandboxed `bash` and write there.
- Seatbelt profiles nest on macOS. Landlock stacks by design and, at ABI 3, does not scope signals.
- `tests/recursion.rs` runs both sandbox cases on macOS locally, and on Linux in CI's `test` job.

## Threat model

Recursion follows `--sandbox`: it stops a model's mistakes, not a hostile model.

- The family's files are owner-only. Another user cannot read them or write to them.
- A child's answer reaches its parent as tool output, so it never carries the user's authority. The parent treats it as it treats any command output.
- No member can widen the root, the sandbox or the writable set it inherited.
- The budget is cooperative. Every member is a `minima` that reserves before each request; a process that skips the ledger is not bounded by it.
- Out of scope: a model that edits the family's files to impersonate a member or erase spend.

## Limits

- **The Chat cap is `max_tokens`.** OpenRouter documents it; Ollama's and llama.cpp's OpenAI-compatible endpoints accept it, unverified here. OpenAI's own Chat endpoint wants `max_completion_tokens` for reasoning models, but minima's `openai` entry uses Responses. A gateway reached through `MINIMA_BASE_URL` may treat the field differently. Only `tests/live_path.rs` checks the field, against a fake provider.
- **A cancel costs its full hold.** Each Esc during a request counts the whole reservation, often tens of thousands of tokens.
- **A child killed by SIGKILL leaves its `bash` jobs running.** Only its own exit path reaches their process groups.
- **`wait` is bounded by the `bash` timeout.** A long child takes several `wait` calls, each a model turn.
- **The REPL has no `/shutdown`.** The user runs `minima -r shutdown` from another terminal.
- **The Linux sandbox tests run only in CI.**

## Tests

| File | Covers |
|-|-|
| `src/family.rs` | answers and `wait`, shutdown scopes, pruning, grants, Busy, forfeits, concurrent reserves, cost limits, name rules, `is_minima`, file modes |
| `tests/recursion.rs` | through the built binary with mock models: answers, the required budget, a root or child crossing the budget, stopping when the parent exits, an agent's `shutdown`, both sandbox cases |
| `tests/live_path.rs` | a family request sends its grant as the output cap on each dialect; outside a family, Chat and Responses send none |
| `scripts/test_recurse.py` | a real model delegates, waits and adds up; spends tokens |

## Rejected

- **A daemon.** It would add a process to supervise and a socket to secure, plus a failure mode files lack: a dead daemon with live agents. Its only gains are mid-turn steering and a mid-stream budget cutoff, neither in scope.
- **Children as tokio tasks.** They would share the parent's provider state, session, sandbox and cancel channel. Separate processes keep each member's state its own.
- **A fifth tool.** The four-tool set in `src/tools/mod.rs` is fixed. `bash` already reaches every command.
- **A mailbox.** The first build had `send` and `recv`, and put mail before the next prompt. That gave a child's text the user's authority, and lost the mail when a run failed before its prompt was pushed. Fork/join needs only the child's answer.
- **A messaging system (minos).** A server, accounts and a broker per agent, for traffic fork/join does not need.

## History

- **2026-11-18:** first build (`f21b122`). It had a mailbox, `send` and `recv`, children that outlived their parents, an optional budget, and signals for `shutdown`. The proposal and that build's decisions are in `git show f21b122:docs/dev/recursion.md`.
- **2026-11-19:** after review. Answers replaced the mailbox; children stop with their parents; the budget became required, then atomic, then granted and waiting; `shutdown` from an agent reaches only its descendants and asks through markers; names are validated. The findings, and what each change answers, are in `review-recurse.md`.
