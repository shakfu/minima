# Recursive agents

How minima could let an agent spawn child agents, see its family, and exchange messages across session boundaries. Proposed 2026-11-18 against minima after session resume landed (`src/session.rs`). Gated behind `-r`/`--recurse`; off by default, so the current single-agent path is unchanged. Implemented 2026-11-18 in `src/family.rs`; where the build differs from the proposal below, [Decisions](#decisions-2026-11-18) records what was built.

Inspired by prime-agent's recursion model. This document scopes recursion to the three primitives that actually make it work, and leaves prime-agent's product features (programmatic tool kernel, continual harness, refinement, heartbeats, MCP, goals) out.

## What "recursive" means here

Three capabilities, no more:

1. **Spawn** a child agent that is itself a full `minima` process with its own session.

2. **A family view** so a running agent can see its parent, siblings and children.

3. **A mailbox** so messages cross session boundaries and are delivered at a turn boundary.

Everything else prime-agent calls recursion (steering a child mid-turn, typed child results, broadcast semantics, model inheritance machinery) is either out of scope or a later step. See [Not proposed](#not-proposed).

## The flag

```text
-r, --recurse    Enable the family subsystem: spawn, send, agents
```

Without `-r`, the family subcommands exit with an error, the mailbox is not read, and no registry entry is written. A session started without `-r` and resumed with `-r` joins the family as a root; a session started with `-r` and resumed without it loses its parent and children for the resumed run only (the registry file is not touched).

Rationale: minima's whole value proposition is "no feature you did not ask for". Recursion adds a cross-process coordination layer; it should not run when a user wants a single agent.

## The family

A **family** is the transitive closure of spawns from one root agent. The root's session id is the **family id**. Every process in the family knows its family id, parent id, own session id and name.

Passed to children through two env vars, which are the cheapest way to cross a `fork`/`exec`:

- `MINIMA_FAMILY` — family id

- `MINIMA_PARENT` — parent session id

A process started without either is a root; it mints a family id equal to its own session id.

### Registry file

One file per family, under `$XDG_STATE_HOME/minima/family/<family-id>.json` (or `~/.local/state/minima/family/...` with the XDG fallback). **Open**: `config_dir()` already carries sessions; putting runtime state next to it would avoid a second directory convention, at the cost of mixing durable data (transcripts) with ephemeral data (pids, mailbox positions). Session resume already chose `config_dir()`; recursion is runtime state, which is what `$XDG_STATE_HOME` exists for. Recommended: state home.

```text
{
  "schema": 1,
  "family": "1763462400-91234",
  "root": "/Users/sa/projects/x",
  "members": [
    { "id": "1763462400-91234", "name": "root", "parent": null,
      "pid": 91234, "session": "sessions/<hash>/1763462400-91234.json",
      "started_at": 1763462400, "ended_at": null },
    { "id": "1763462461-91237", "name": "reviewer", "parent": "1763462400-91234",
      "pid": 91237, "session": "...", "started_at": 1763462461, "ended_at": null }
  ]
}
```

- Written with the tmp-plus-rename from `state.rs::store`, under 0700/0600 like sessions.

- Updated on spawn (append member), exit (set `ended_at`), and send (no update; the mailbox is a separate file).

- Concurrent writers coordinate through a file lock on `<family-id>.lock`. **Open**: `fs2`-style `flock`, or rely on the tmp-plus-rename being atomic and tolerate lost-update of `ended_at`. Recommended: a short `flock` around read-modify-write. Three or four processes contending is the realistic case.

### Status

`pid alive && ended_at == null` → running. `pid dead && ended_at == null` → crashed; shown as `exited`. `ended_at != null` → `exited` with timestamp. No heartbeat; the pid check is enough for the accident model, same as sessions' mtime ordering.

## The mailbox

One JSONL file per session member: `$XDG_STATE_HOME/minima/mailbox/<session-id>.jsonl`. Each line is one message:

```text
{"from":"<session-id>","from_name":"reviewer","at":1763462500,"text":"..."}
```

- **Writers** append under an `O_APPEND` open, which is atomic for writes under `PIPE_BUF` on POSIX. Messages exceeding that are chunked by the sender into a single JSON line plus newline; a reader that sees a partial line waits for the next drain. Simpler alternative: short `flock` around the append, matching the registry. **Open**: `O_APPEND` only, or `flock`-guarded append. Recommended: `flock`. One extra syscall per message is irrelevant; a half-written line is not.

- **Readers** drain the file at a turn boundary (see below), truncate it to zero, and inject each line as one user-role message into `Agent::messages` with a prefix like `[from <name>]`.

- A mailbox for a session that no longer exists is garbage-collected when the registry prunes its member.

### When drains happen

In `Agent::run`, before building the request for a fresh user turn. Not between provider round-trips inside a turn, because a message arriving mid-tool-call changes what the model already decided to do. This matches prime-agent's "messages arrive at turn boundaries" rule without needing the `Cancel` wiring that steering would require.

A drain that finds messages does not auto-start a turn in the REPL. It prints `[1 message from reviewer]` and the user decides whether to send a prompt. In `-p` mode (one-shot), the drain runs once before the single turn and once after, so a late reply can land in the session transcript.

## The subcommands

Not new tools. The README's four-tool freeze stands. The agent reaches recursion through `bash`, like it reaches `git` or `cargo`.

```text
minima spawn    --name NAME [-m MODEL] [-P PROVIDER] -- "prompt"
minima send     --to NAME_OR_ID "text"
minima agents   [--prune]
minima shutdown [--family ID] [--grace SECS]
```

- **`spawn`** forks a detached `minima --recurse -p "prompt"` in a new process group, with `MINIMA_FAMILY` and `MINIMA_PARENT` set, inheriting the parent's `--sandbox`, `--root` and `--writable` set verbatim. Returns the child session id on stdout. Non-blocking: the child runs headless and writes its reply to its own mailbox when done, so the parent reads it with `send` + a later drain, or by reading the child's session file directly.
- **`send`** appends one line to the target's mailbox. The target is resolved against the registry by name first, then by session id. Sending to a dead member fails loudly; the message is not silently dropped.
- **`agents`** prints the registry as a table: `name  role  status  model  session`. `role` is `parent`, `self`, `sibling`, `child`. `--prune` deletes exited members from the registry and their mailbox files; without it the registry grows forever.
- **`shutdown`** forces the whole family down. See [Forced shutdown](#forced-shutdown).

Names are unique among siblings only, matching prime-agent. The registry enforces this on spawn.

### Forced shutdown

Ctrl-C in a parent REPL kills only that process's group. Children are in their own groups by design, so a parent exit leaves them running as orphans and no signal from the parent reaches them. The registry is the only thing that knows the family exists, so family-wide shutdown is a subcommand that walks it.

`minima shutdown`:

1. Resolves the family: `--family ID`, else `MINIMA_FAMILY` from the environment, else the registry whose `root` matches the cwd. Errors if none resolves.
2. Walks the members where `ended_at == null` and `pid` is alive. For each:
   1. Verifies the pid is actually a `minima` process — `/proc/<pid>/comm` on Linux, `ps -p <pid> -o comm=` on macOS. A pid recycled into something else is skipped with a note. This is the only defence against a stale registry after a reboot.
   2. Sends `SIGTERM` to the process group (`kill(-pid, SIGTERM)`). Since each child is a group leader, the signal also reaches that child's `bash` descendants, which is what the existing single-agent cancel path already relies on.
   3. Waits up to `--grace` seconds, default 5, matching the exit-signal path in `src/main.rs`.
   4. Sends `SIGKILL` to the group if still alive.
   5. Sets `ended_at` and writes the registry.
3. Prunes exited members and their mailbox files on the way out, so a successful `shutdown` leaves no trace of the family.
4. Exits non-zero if any member resisted `SIGKILL`. That should not happen; if it does, it is a bug worth seeing.

`shutdown` is a command, not a signal. Ctrl-C in the root REPL stays local. Wiring Ctrl-C to take the family down conflicts with the "orphans survive" rule the rest of the design relies on, and is not proposed.

### Exit hook

Under `--recurse`, every process installs a `SIGTERM`/`SIGINT` handler that sets its own `ended_at` and releases the mailbox lock before exit. The normal-exit path already does this for clean exits; the handler is the `SIGTERM` path `shutdown` itself relies on. Without it, a forced shutdown leaves a wall of `exited` entries with no end time.

### What shutdown does not solve

- **A child stuck in a provider call.** `SIGTERM` interrupts the tokio runtime; `SIGKILL` always wins. Same story as single-agent minima today.
- **A grandchild whose parent exited before `shutdown` ran.** Still in the registry, still reached, because the family is flat in the file.
- **Another user's family.** 0600 files make the registry unreadable to other users; `shutdown` sees only its own.

## Sandbox inheritance

**Settled.** A child inherits `--sandbox` and the `--writable` set from its parent, verbatim. No way to widen either from `spawn`. Narrowing is also not offered in the first cut; if a child wants a narrower root, the user can start a separate family.

Consequences:

- `spawn` reads bounds from the parent's own `Cli`/`Bounds`, not from flags on the `spawn` invocation. `--sandbox` or `--writable` passed to `spawn` is an error.

- A sandboxed parent cannot create a child that escapes the root. The kernel policy applies to `spawn` itself (it runs under `bash`) and to every descendant.

- The child's `--root` equals the parent's canonical root. A child cannot `cd` outside it via spawn arguments.

This is the cheapest rule and the only one that matches the accident model recorded in `root-sandbox.md`: a sandbox exists to stop mistakes, so widening it from inside the sandboxed process would defeat the mechanism.

## Budget

A family can be given a token or cost budget on the root. Members check it at turn boundaries and force shutdown just before crossing. No daemon.

### The flags

```text
--budget-tokens N    Total family tokens (input + output). Family shuts down before N is crossed.
--budget-cost USD    Total family cost in USD, same semantics.
```

Either or both; the family stops when the first one would be crossed. Cost uses the pricing already in `src/price.rs`. The numbers travel to children in `MINIMA_BUDGET_TOKENS` and `MINIMA_BUDGET_COST`, so a child inherits them without re-reading a config. A child cannot widen them, same rule as the sandbox.

### The ledger

One file per family, next to the registry:

```text
$XDG_STATE_HOME/minima/family/<family-id>.ledger.jsonl
```

One JSON line per completed round-trip:

```json
{"member":"<session-id>","at":1763462500,"tokens_in":1240,"tokens_out":380,"cost_usd":0.0071}
```

Append-only, `flock`-guarded like the mailbox. A `shutdown --prune` deletes it with the rest of the family's files.

### Where the check runs

In `Agent::run`, in the same precheck spot as the context-window check, *before* each provider request:

1. Read the ledger (small, append-only; read the whole file).
2. Sum `tokens_*` and `cost_usd` across every member.
3. Add this member's `used + pending + max_output` as the upper bound of the request about to be sent.
4. Compare to `budget * (1 - RESERVE)`. Recommended reserve: 5%.
5. If the sum would cross the line:
   - print the overrun (to the REPL, or to the `-p` result in headless mode),
   - call `minima shutdown --family <id>` on the family,
   - exit non-zero.

After each successful round-trip, append the actual usage to the ledger.

The "just prior to exceeding" guarantee is that the sum **after the next request completes** is compared, not the sum right now. That is the honest definition without a daemon: budget is enforced at turn boundaries, with `max_output` treated as the possible overshoot per turn.

### What this catches

- A family slowly burning budget across many members. Caught on the next precheck anywhere.
- A child that spawns more children. Each new member appends to the ledger; each precheck sees the new total.
- Two members noticing overrun at once. Both call `shutdown`; `shutdown` is idempotent — registry writes are `flock`-guarded, `SIGTERM` to an already-dead pid is a no-op.

### What this does not catch

- A single response larger than the per-request `max_output` suggests. Minima already caps output per request; the budget treats that cap as the overshoot bound. Shrinking the reserve shrinks the overshoot at the cost of stopping work short of the budget.
- Mid-stream enforcement. Interrupting a half-streamed response needs either a daemon or signalling into a streaming provider call, which is exactly the steering case the main design declined. The next precheck catches the overrun.
- A process stuck in a long `bash` call. Bounded by the existing bash timeout (120s default, 600s cap), not the budget.

### Honest caveat

A soft budget, not a hardware limit. It assumes every member is a cooperating `minima` process that will check the ledger. Under the accident model this holds — a model that forgets to check is the single-agent case, which has no budget today. A model that patches out the check is out of scope, same reason `--sandbox` is accident-only.


## Changes in the loop

Localised, in roughly this shape:

- `src/config.rs`: `recurse: bool` on `Cli` and a new `Family` field on `Bounds`, or a sibling struct. `Family` carries family id, parent id, own id, name.

- `src/main.rs`: when `recurse`, read `MINIMA_FAMILY` and `MINIMA_PARENT`, register as a member, install an exit hook that sets `ended_at`.

- `src/agent.rs`: one call to `family::drain(&session_id)` before each fresh user turn, inside `Agent::run`. Messages come back as `Role::User` with a `[from <name>]` prefix.

- `src/prompt.rs`: when `recurse`, append a short block to the system prompt naming the agent's role, name, parent and siblings. Omitted when `recurse` is off, so no prompt churn for existing users.

- New `src/family.rs`: registry read/write, mailbox append/drain, spawn helper. Estimated 300-400 lines.

- `src/main.rs`: three `clap` subcommands dispatched before the normal REPL path.

Estimated total: 700-900 lines, about one-fifth of the current tree, concentrated in one file. The extras over the base family subsystem are `shutdown`, `agents --prune`, the signal handler, and the budget ledger (~85 lines in `src/family.rs`, ~15 in `src/agent.rs`, ~10 in `src/price.rs`).

## Interaction with session resume

- `--continue` and `--resume ID` work with `--recurse`. A resumed session rejoins its family if the registry still lists it with the same id. If the registry file is gone, the session becomes a new root.

- A resumed session's pid changes; the registry is updated on resume.

- `--no-session` with `--recurse` is allowed. The family view and mailbox still work for the lifetime of the process; nothing is persisted beyond the registry's own records, which are not transcripts.

## Threat model

Recursion is for accidents, like `--sandbox`. Specifically:

- The registry and mailbox are user-owned, 0600. Another user on the same host cannot read them, so cross-user message injection is not in scope.

- A compromised child can send any text to any family member's mailbox. The parent sees messages tagged with sender name and id and treats them as untrusted input, same as any tool output. No capability is granted by receiving a message; the parent still decides what to do.

- `spawn` cannot widen sandbox bounds. A child that calls `spawn` again is bound by the same rules it inherited.

- Out of scope: a model that races the registry file against a concurrent writer to impersonate a sibling. The `flock` makes it hard; truly adversarial models are out of scope for `--sandbox` already.

## Not proposed

- **Steering.** No interrupting a child mid-turn with a parent message. Delivery is at turn boundaries. A later step could wire `Cancel` through the mailbox drain, at the cost of the "model already decided" problem above.

- **Typed child results.** A child's reply is text in the parent's mailbox, or the child's session transcript on disk. No structured return value, no `collect`, no promise handles.

- **Broadcast.** `send` takes one target. A loop over `agents` output is the cheap substitute.

- **A daemon.** Everything is files and processes. If a parent exits, children keep running as orphans and their pids are still valid; the next `agents` call in any member sees them as `running` with `parent: null`.

- **Continual harness, refinement, heartbeats, goals, MCP, programmatic tools.** These are prime-agent product features, not recursion primitives.

- **Model/provider inheritance machinery.** `spawn` takes `-m` and `-P` or inherits from the parent's own config. No per-family defaulting rules.

- **Herdr integration.** Herdr already arranges panes; a family member is not a pane. Out of scope.

## Open questions

1. **Registry location**: `$XDG_STATE_HOME/minima/` or `config_dir()/family/`. Recommended: state home. Settles with one line in `src/family.rs`.

2. **Mailbox atomicity**: `O_APPEND` alone or `flock`-guarded append. Recommended: `flock`, matching the registry.

3. **Registry locking**: `flock` on a `.lock` file, or rely on tmp-plus-rename and accept lost-update of `ended_at`. Recommended: `flock`.

4. **System-prompt block under `--recurse`**: exact wording and length. A one-line sentence plus the `agents` output is probably enough; measure once a first version runs.

5. **Spawn stdout**: the child session id only, or id plus mailbox path. Recommended: id only. The mailbox path is derivable.

## Cost

Estimated, not measured:

- ~500-700 lines, concentrated in `src/family.rs`, plus about 20 lines in `src/agent.rs`, 30 in `src/main.rs`, 10 in `src/config.rs` and 15 in `src/prompt.rs`.

- No new dependency if `flock` is reached through `nix` (already in the tree via `sanduk-sandbox`). Otherwise `fs2` or `rustix::fs::flock`.

- Tests:
  - round trip through the registry file

  - `shutdown` SIGTERMs live members, SIGKILLs after grace, updates `ended_at`

  - `shutdown` skips a pid recycled into a non-minima process

  - the SIGTERM handler sets `ended_at` before exit

  - `agents --prune` removes exited members and their mailbox files

  - budget precheck refuses a request that would cross the line and triggers shutdown

  - post-turn ledger append is seen by a sibling's next precheck

  - a 5% reserve stops work before the hard line

  - a family with no budget flags skips the ledger entirely


  - mailbox drain injects messages in order

  - `spawn` without `--recurse` on the parent errors

  - `--sandbox` and `--writable` are inherited; passing either to `spawn` errors

  - a dead member reports `exited`; sending to it errors

  - resume with the registry still present rejoins the family

  - resume with the registry gone starts a fresh root

  - the mode of registry and mailbox files is 0600

## Alternatives considered

- **A long-running daemon.** Rejected: adds a process to manage, a socket to secure, and a failure mode (daemon dead, agents live) that files do not have. Prime-agent has a daemon because it also runs the RLM kernel; minima has no equivalent.

- **In-process children with Tokio tasks.** Rejected: a child sharing the parent's address space shares its provider state, its session file, its sandbox installation and its cancel channel. Separation across processes is what makes the family tree a tree.

- **`bash minima -p ...` with no infrastructure.** Already works today and is the honest baseline. It gives no family view, no async messaging, no identity: a one-shot subprocess with a transcript nobody references. Recursion as proposed here is the next step up and the smallest one that is actually useful.

- **A fifth tool, `agent`, with subcommands.** Rejected: the four-tool freeze in `src/tools/mod.rs` exists for a reason. Shelling out through `bash` matches how the agent already reaches `git`, `cargo` and `rg`.

## Decisions (2026-11-18)

The five Open questions took their recommended answers: the state directory, `flock` for the mailbox, `flock` for the registry, a short system-prompt block, and the bare id on `spawn`'s stdout.

Changed from the proposal:

- **Subcommands follow `-r` directly**: `minima -r spawn ...`. Bare `minima spawn ...` would have turned `minima send the logs to x` from a prompt into a command for every user, with or without `-r`.
- **`recv [--wait SECS]` added.** Turn-boundary drains reach a REPL user's next prompt, but a `-p` agent has no next prompt: without `recv` it could spawn children and never read their answers. `--wait` is bounded by the `bash` timeout.
- **One directory per family**: `<state>/family/<family id>/` holds `registry.json`, `ledger.jsonl`, `<member>.mail`, `<member>.log` and `lock`. One `flock` covers all of them; deleting the directory deletes the family.
- **The family id is the root's member id**, `<unix seconds>-<pid>`, not its session id. The member id is fixed before the session is known. A spawned child's session id is its member id, so `minima --resume <id>` opens a child's transcript.
- **No rejoin on resume.** `--resume` with `-r` founds a new family. Rejoining needed the registry to outlive the processes, which pruning does not allow.
- **The registry, not the environment, carries what children inherit**: root, sandbox, writable set and budget. The environment carries only identity (`MINIMA_FAMILY`, `MINIMA_SELF`, and for a starting child `MINIMA_PARENT`, `MINIMA_ID`, `MINIMA_NAME`, which it removes from its own environment). A child given `--budget-*` refuses to start.
- **Under `--sandbox`, the family directory joins the writable set.** The subcommands run under the sandboxed `bash` and must write there. A sandboxed child saves no session (`--no-session`), since the session directory is outside the writable set.
- **Nested sandboxes work on macOS.** A child installs its own Seatbelt profile inside the parent's `bash` sandbox; measured 2026-11-18 on this machine: its write under the root succeeded, its write to `/etc` was denied. Landlock stacks by design. Untested on Linux.
- **A child's final answer goes to its parent's mailbox**, as `[message from NAME (ID)]` before the parent's next prompt, or through `recv`. A failed or cancelled run sends `failed: ...` or `cancelled`. The REPL prints no notice when mail arrives.
- **Children inherit provider and model** from the registry, not `--base-url`, `--api-key` or `--context` flags; their environment variables pass through as the process environment does. `spawn --mock PATH` exists for tests.
- **The budget projection** is `context - room` input plus `min(max_output, room)` output, with Anthropic's 32,000-token default where the model list gives no ceiling. `room` is what `one_turn` already receives, so the main loop and the compaction request are checked at one place. The cost projection needs a price list; with a provider that reports cost but has no listed rates, only spent cost is checked.
- **The ledger is written only when a budget is set.** Nothing else reads it.
- **The member that detects the overrun survives.** It signals every other member, then fails the request; it refuses each later one the same way. A REPL root stays open with its children gone.

Measured end to end with mock providers (`tests/recursion.rs`): a child's answer reaches its parent through `recv`; a root crossing a 40,000-token budget stops a child in the middle of `sleep 30`, the child's exit hook kills the `sleep`, and the child is pruned from the registry, within about three seconds.

Not done:

- No REPL `/shutdown`. `!` is not a REPL feature; the user runs `minima -r shutdown` from another terminal or the agent does through `bash`.
- A child killed by `SIGKILL` after the grace period leaves its `bash` jobs running: they are in their own process groups, which only its exit hook reaches.
- Cost: 887 lines in `src/family.rs` plus 163 of its tests, 85 in `src/agent.rs`, 70 in `src/main.rs`, 15 in `src/config.rs`, and 155 in `tests/recursion.rs`. Above the 700-900 estimate, mostly from `recv`, the subcommand parser and rustfmt's layout.
