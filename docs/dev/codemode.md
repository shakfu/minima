# Codemode with rquickjs

Whether minima should add codemode: a tool that runs model-written JavaScript in QuickJS through [rquickjs](https://crates.io/crates/rquickjs), with minima's tools callable from the script. Written 2026-10-15 against rquickjs 0.14.0 and minima 0.6.0. Status: optional codemode rejected 2026-10-24, see "Conclusion"; `js-only` untested on fix tasks. A system-prompt line steering aggregation into `bash` is in as a separate trial; it does not settle codemode. The strongest case found is a run without `--sandbox`, below.

## What codemode is

The model writes a short script instead of a sequence of tool calls. The script calls tools, filters their output and returns only the result. Intermediate output never enters the context window.

Pi 1.0 ships it ([Ronacher, "What is Codemode"](https://lucumr.pocoo.org/2026/10/6/codemode/); details below are from a [secondary summary](https://braindetox.kr/en/posts/codemode_tool_calls_as_code_2026.html), not the original):

- QuickJS runs inside a WASM runtime on the harness side. No filesystem, network or timers.

- Scripts call `tools.bash(...)`, MCP tools (`tools.mcp__<server>__<tool>`), model APIs, and `store()`, which saves data to the transcript for a later script.

- `async`/`await` and `Promise.all` work. Tool calls run at most four at a time.

- MCP tools stay out of the context; the system prompt lists each server in one line. This is Pi's main reason for codemode.

- No token-saving figures are published. A widely repeated "40%" has no source. The summary says the pattern works poorly with smaller models.

## Fit with minima

- **MCP is absent.** minima has four tools and no MCP, so Pi's main motive does not apply.

- **`bash` already composes.** Loops, filters and aggregation run today as pipelines, `jq`, `awk` or `python -c`, under the kernel sandbox. Codemode adds two things over that: typed `read`/`edit` calls across many files in one turn, and parallel calls. `src/agent.rs` runs a turn's calls one at a time.

- **Exposing `bash` to scripts voids QuickJS isolation.** The boundary is then the tool set plus `sanduk-sandbox`. QuickJS isolation matters only if scripts get a strict subset of tools.

- **The four-tool freeze.** `src/tools/mod.rs` and `bashkit.md` treat a fifth tool as a cost. rquickjs also adds a C build through `cc`, compile time and binary size (not measured).

## Native rquickjs versus isolation

`sanduk-sandbox` confines only the `bash` child (`root-sandbox.md`). minima's own process is unconfined and holds API keys. Native rquickjs runs QuickJS's C code in that process, so a memory-safety bug in the engine executes outside `--sandbox`.

`--sandbox` is scoped to a model's mistakes, not a hostile model (CHANGELOG, Unreleased). An accidental script does not exploit a heap bug, so in-process QuickJS fits that scope. It stops fitting if the threat model ever widens to prompt injection through file contents or tool output.

| | Native rquickjs, in-process | rquickjs in a sandboxed child | QuickJS in WASM (`wasmtime`) |
|-|-|-|-|
| Engine bug reaches | minima's process | the child, under the `bash` policy | the WASM instance |
| Extra code | least | IPC for tool calls, est. 150-250 lines | `wasmtime` plus a `.wasm` build |
| Matches Pi | no | no | yes |

## Without `--sandbox`

Without `--sandbox`, `bash` runs unconfined. This happens when the user omits the flag, and when `preflight` fails (kernel without Landlock, a container that blocks it) and the error says to run without it.

In that mode, codemode with a restricted tool set is a confined channel that `bash` cannot be:

- QuickJS has no filesystem, network or process access. A script reaches only the tools passed in.
- Pass `read`, `write` and `edit`, not `bash`. Apply `confine_path` to `write` and `edit` inside codemode whether or not `--sandbox` is set.
- Multi-file reading, filtering and edits then run with writes bounded to the root and no process spawned. Today the same work goes through unconfined `bash`, and the prompt line pushes more of it there.
- In-process QuickJS adds little exposure here: unconfined `bash` already runs with the same privileges.

Limits:

- `bash` stays available, and nothing makes the model prefer codemode for work `bash` can also do. A "no-`bash`" mode that offers only `read` and codemode, plus confined `write` and `edit`, would make the channel enforced instead of advisory. It also loses builds and tests.
- `read` stays unbounded, so the restricted set confines writes only. Reading `~/.ssh` is still possible, with no network to send it out except through the model's reply.

## Defects in the commonly suggested harness

A sample harness from a search-engine answer has these problems. Any implementation should avoid them.

1. The interrupt deadline is captured once at runtime creation, so every eval after the first timeout window is interrupted immediately. The deadline must be set per eval.

2. Tools are called synchronously. minima's tools are `async`; `block_on` inside tokio panics or deadlocks. Parallel calls need the `futures` feature with `AsyncRuntime` and `AsyncContext`.

3. `Runtime` is `Send` only with the `parallel` feature ([docs.rs](https://docs.rs/rquickjs/0.14.0/rquickjs/struct.Runtime.html)).

4. The interrupt handler must also poll `Cancel`, or Esc does not stop a script.

5. Results cross the boundary as JSON strings parsed again in JS, and the return value is formatted with `{:?}`. `json_parse` and `json_stringify` on the context do both properly. Output still goes through `tools::cap`.

6. A `HashMap<&str, Arc<dyn AgentTool>>` registry behind a mutex duplicates the `Tool` enum.

## Options

1. **Prompt only (in, as a trial).** `src/prompt.rs` tells the model to filter and aggregate inside one `bash` command. Zero dependencies. Confined only as far as `bash` is: fully under `--sandbox`, not at all without it.

2. **Codemode in a sandboxed child.** `minima --js-worker` under the `bash` policy, tool calls over stdio.

3. **Codemode in WASM.** Pi's design. Strongest isolation, largest dependency.

4. **Codemode replaces the four tools** with one `run(js)`. A different product: the model loses direct tools it handles reliably.

## What would justify codemode

- Sessions where repeated `read` or `bash` output dominates input tokens despite the prompt line. Per-turn usage is in `--json` output.

- MCP support. Codemode is then the way to keep tool schemas out of the context.

- A need for parallel tool calls that `bash` backgrounding (`&`, `wait`) does not meet.

- Runs without `--sandbox` where multi-file work currently goes through unconfined `bash`.

## Experiment

Built as `cargo build --features codemode`. Off by default; the default build is unchanged. `MINIMA_CODEMODE` picks the variant at startup, and a misspelt value fails the run.

| Variant | Direct tools | Scripts reach | Aggregation line |
|-|-|-|-|
| `none` | read, write, edit, bash | - | none |
| `line` (default) | read, write, edit, bash | - | use `bash` |
| `js` | + `js` | read, write, edit, ls | use `js` |
| `js-only` | read, write, edit, `js` | read, write, edit, ls | use `js` |
| `js-bash` | + `js` | read, write, edit, ls, bash | use `js` |

Implementation (`src/tools/js.rs`):

- Native rquickjs, without the `futures` feature. Each call gets a fresh runtime on its own thread. Tool calls block on the tokio handle from that thread, so scripts stay synchronous and no worker stalls.
- `tools.all([[name, args], ...])` runs calls concurrently with `join_all`. This replaces `Promise.all`.
- Limits: 64 MiB heap, 1 MiB stack, per-call deadline (default 120 s, max 600 s). The interrupt handler also polls `Cancel`.
- Script `write` and `edit` always go through `confine_path`, with or without `--sandbox`.
- `scripts/codemode_bench.py` runs five checked tasks under each variant on a fresh `git archive HEAD` copy. It records pass rate, tokens, turns, cost and calls per tool. By default it runs without `--sandbox`.

Findings so far, from building it:

- **The tool cap breaks aggregation.** The first version passed script reads through the `read` tool, which stops at `TOOL_OUTPUT_CAP` (32 KiB). A line count of `agent.rs` returned 732 of 1206 lines, with no error. Scripts now read the whole file, up to 8 MiB; only the script's result is capped. Inner `bash` output in `js-bash` is still capped.
- **`js-only` needs a way to list files.** Without `bash`, the four tools cannot list a directory, so scripts also get `tools.ls(dir)`. That makes the variant a fifth capability, not only a new channel.
- **Inner calls are invisible.** The user sees one `js` call; reads and edits inside it are not shown.
- **No state between calls.** Pi's `store()` has no equivalent here.
- **Cost of the dependency.** The release binary grows from 6.5 MiB to 7.6 MiB on macOS arm64. rquickjs-sys 0.14.0 vendors quickjs-ng 0.16.2 (`QJS_VERSION_*` in its `quickjs/`), not Bellard's QuickJS.

### Results: Qwen3.5-9B, local (incomplete)

2026-10-15, Q4_K_M under llama.cpp (`llama serve -c 32768 --reasoning off`), M1 16 GB, no `--sandbox`, one rep. About 90 s per run. The run was stopped after the pilot and four `count-tests` runs.

| task | variant | pass | input tokens | turns | calls |
|-|-|-|-|-|-|
| control | line | 1/1 | 3432 | 2 | read=1 |
| control | js | 1/1 | 2060 | 2 | bash=1 |
| count-tests | none | 0/1 | 220588 | 30 | bash=30 (hit `--max-turns`) |
| count-tests | line | 0/2 | 25747, 60844 | 8, 12 | bash only |
| count-tests | js | 0/2 | 11035, 3877 | 6, 3 | bash only |
| count-tests | js-only | 0/1 | 42656 | 8 | bash=7, all refused |

- The model never called `js`, in any variant.
- In `js-only` it called `bash`, which was not advertised, seven times. Every call was refused, and it still answered with per-file counts it had no way to obtain.
- Every `count-tests` answer was wrong (the true total is 206; grep agrees with the checker).
- So at 9B the variants do not differ in the way that matters, because codemode is never tried. Token differences across variants come from different `bash` strategies.

### Results: Claude Haiku 5.5 and Sonnet 5.5

2026-10-24, `anthropic` provider, no `--sandbox`, one rep per cell. Haiku: 26 runs; the pass was killed after 11 and resumed, so `doc-gaps`/`none` ran twice. Sonnet: 25 runs.

| Variant | Haiku pass | Haiku `js` calls | Sonnet pass | Sonnet `js` calls |
|-|-|-|-|-|
| `none` | 6/6 | - | 5/5 | - |
| `line` | 5/5 | - | 5/5 | - |
| `js` | 5/5 | 0 of 8 | 5/5 | 1 of 6 |
| `js-bash` | 5/5 | 0 of 8 | 5/5 | 2 of 6 |
| `js-only` | 4/5 | 6 of 7 | 5/5 | 5 of 5 |

- Neither model chose `js` for a task one `bash` call could do. Sonnet's 3 `js` calls were all on `doc-gaps`, where attributes between `///` and `pub fn` make a one-liner awkward.
- On `doc-gaps`, Sonnet's optional `js` cost more than `line`: 6657 and 6170 input tokens against 5031, at 3 turns each. The model explored with `bash` first. `js-only` used 3327 tokens in 2 turns, the lowest of any variant.
- On the other four tasks, `js-only` cost Sonnet 7-80% more than `none`. A script takes more output tokens than a pipeline.
- Offering `js` adds 700-800 input tokens per request, from the schema and the aggregation line. On Sonnet that is 10-15% of a simple run's cost, paid whether or not `js` is called.
- Haiku's one failure was in `js-only`: all three line counts one high (1207 for 1206), consistent with counting `split('\n')` elements.
- Four of five tasks finish in one call and two turns under every `bash` variant. These tasks cannot show an efficiency gain over `bash`.
- Reported cost: Sonnet $0.22. Haiku $0.0145, about 5x below what its token counts imply at list price; the price-list match for `claude-haiku-5-5` is unverified.

### Conclusion

Optional codemode (`js`, `js-bash`) is not added. Three models rarely or never chose `js` over `bash`, no run measured a gain from it, and every request pays for its schema. The case in "Without `--sandbox`" needs the model to route edits through `js`, and none did.

`js-only` stays open as a separate product. It won the one structured task and lost the simple ones. Its main cost, no builds or tests, is unmeasured; `scripts/history_bench.py` under `MINIMA_CODEMODE=js-only` and `line` with a codemode build would measure it.

The spike stays behind `--features codemode`. Default builds are unchanged.

### Resuming

```
cargo build --features codemode
ANTHROPIC_API_KEY=... scripts/codemode_bench.py --provider anthropic \
  --model <model> --reps 3 --out codemode-bench.jsonl
scripts/codemode_bench.py --summarise codemode-bench.jsonl
```

`--tasks` and `--variants` narrow the run; `--context` is needed for a local server. Each JSONL row holds the answer text, so failures can be read after the run.

Open:

- `js-only` on fix tasks, per "Conclusion". More reps or harder tasks matter only for `js-only`.

## Not verified

- Ronacher's original post. Pi details rest on one summary.

- rquickjs build time. Two cold release builds (1m43s without, 1m08s with) ran in separate target directories under different load, so they do not compare.

- Whether rquickjs catches a Rust panic in a host callback before it crosses into C.

- That the prompt line changes model behaviour. Measure before and after over a few sessions.
