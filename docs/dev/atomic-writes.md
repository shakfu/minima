# Confining write and edit

How `write` and `edit` could be enforced by the kernel under `--sandbox`, instead of by a path check in minima's own process. Proposed 2026-09-23 against minima 0.5.0, after a self-review flagged the race. Not implemented.

## The problem

Under `--sandbox`, `write` and `edit` are bounded by `confine_path` (`src/tools/mod.rs:138`). It resolves the path, checks it, and returns it. `atomic::replace` then opens the path again by name: `create_dir_all`, the temp-file create, and `rename` (`src/tools/atomic.rs:11-49`). All of this runs in minima's unconfined process.

Between the check and the open, a background job started through `bash` can replace a directory on the path with a symlink to one outside the root. The write follows it. `root-sandbox.md` already records this window (line 65) and accepts it (line 126).

It matters for threat model 2 in `root-sandbox.md`, an adversarial model. Under `--sandbox` the kernel confines `bash`, so this race is that model's one remaining way to write outside the root. A model that only errs does not produce the timing.

## Proposal

Run the write in a child process under the same kernel policy as `bash`. Keep the path check.

1. Add a hidden subcommand, `minima __replace <path>`. It reads the contents from stdin and calls `atomic::replace`.
2. Under `--sandbox`, `write` and `edit` spawn that subcommand through the platform sandbox. `edit` still reads and matches in the parent; only the replacement moves.
3. `confine_path` stays. It enforces two rules the kernel policy does not: the root alone, where the policy also allows `$TMPDIR`, `/dev/null` and the build caches, and the `.git`/`.env` protection. It also gives the model a clear refusal for an ordinary wrong path.

With `--sandbox` off, nothing changes.

`sandbox_command` (`src/tools/bash.rs:413,481`) currently hard-codes `bash -c <command>`. It needs to take a program and its arguments. Both platforms already allow this: Landlock is applied in `pre_exec` and holds for any program, and Seatbelt uses the `sandbox-exec -p <profile>` prefix.

Result:

- The kernel resolves the path at open. The check-to-use window closes.
- A swap during the window can at most move the write into `$TMPDIR` or a cache. `bash` can already write there.
- `atomic.rs` does not change. There is one implementation of the atomic write.
- The kernel enforces both places the policy is enforced, instead of one.

The subcommand exists to give the sandboxed child a program to run. minima's own process cannot be confined, because it writes the config directory, the model cache and the prompt history.

## Alternatives

- **Document only.** A README note on the window. This fits if `--sandbox` targets errors alone. Then `root-sandbox.md` should drop threat model 2 rather than leave it half closed.
- **`openat2` with `RESOLVE_BENEATH`.** Linux only; macOS has no equivalent. Every step of `atomic::replace` would have to use directory handles: `mkdirat`, `openat`, `fchmod`, `renameat`. The two platforms would then give different guarantees.
- **Landlock on a dedicated thread.** Landlock restricts the calling thread, so one OS thread could restrict itself, write, and exit, without a spawn. Seatbelt restricts the whole process, so macOS would still need the child. That means two mechanisms.
- **Sandboxed shell.** Pipe the contents to `bash` under the existing policy and `mv` a temp file into place. This avoids the subcommand, but moves fsync, the mode copy and symlink-following from `atomic.rs` into shell. That is a second implementation, with quoting risk.

## Costs

- One process spawn per `write` or `edit` under `--sandbox`: a few milliseconds.
- About 10 lines of argument dispatch in `main.rs`, plus passing the child's error back as the tool result.
- A test that swaps a directory for a symlink between the check and the write, and asserts that nothing is written outside the root. Today no test covers concurrent changes to the filesystem.

## The binary itself

The child runs `current_exe()`. When minima runs as `./minima` from inside the root, sandboxed `bash` can replace that file. The child is sandboxed, so a replaced binary can do no more than `bash` can. The next unsandboxed launch is a separate exposure, and this proposal does not change it. On Linux, exec `/proc/self/exe` instead: it names the running inode even after the path is replaced.
