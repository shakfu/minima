# Confining the bash tool

Whether `bash` can be confined to one directory, and whether a path guard on the other tools is worth having without it. Written 2026-09-19 against minima 0.3.0. The implementation now uses both: structured tools enforce the path policy, and `bash` uses the platform sandbox.

`--root DIR` defaults to the working directory. `read`, `write`, and `edit` reject paths outside it. `bash` applies Landlock on Linux and Seatbelt on macOS.

## The problem

A path guard in minima's own process can refuse `write` and `edit` outside one directory. `bash` ignores it, because a shell can run any program on the machine.

Two threat models:

1. A model that errs. It writes to the wrong path by mistake. A path guard refuses this for `write` and `edit`.

2. A model that is adversarial, or a prompt injection. It intends to leave the directory. A path guard refuses nothing, because `bash` is one call away.

Only an operating system facility closes case 2 for `bash`.

## Why a path guard on the other tools is not worth having alone

minima has four tools (`src/tools/mod.rs`), and the freeze at four is what buys the enum over
`dyn Tool`. None of them renames, deletes or creates a directory. So `rm`, `mv`, `mkdir`,
`git checkout` and `git apply` can only go through `bash`. That is forced by the tool set, not a
model habit.

A guard on `write` and `edit` therefore prevents a file being created in the wrong place. It does
not prevent `rm -rf ~/notes`. The destructive operations are exactly the ones it cannot see.

Constraining what the model generates does not help either, because the constraint is semantic
rather than syntactic. Where a path resolves is a runtime fact.

- Prompt and tool-description steering is probabilistic. Measured in this project on an easier
  rule: GPT models wrapped commands in `bash -lc` despite the tool description, in 2 of 7 live
  runs of `gpt-5-nano` (CHANGELOG, 0.3.0).
- Constrained decoding needs logit access, which no provider API exposes. A grammar constrains
  syntax; it cannot express "this path resolves inside the root".
- Validating the generated string is the analysis the next section shows is defeated.

## Why wrapping the command string cannot work

Each of these escapes a path test on the command text:

- `eval "$(printf '\x63\x61\x74')"`, `${x:0:1}` assembly, `$IFS` splitting

- `printf x > "$(echo /et''c/foo)"`, where the path never appears in the text

- `python3 -c 'open("/etc/x","w")'`, and the same in `perl`, `node`, `awk`

- `sed -i`, `tee`, `dd of=`, `cp`, `install`, `truncate`, `git checkout`

- `exec 3>/etc/x; echo y >&3`

- writing a script inside the root, then running it

An allow-list of programs is the only version that resists these. It still fails against any interpreter on the list, and it removes most of what the tool is for.

## The design: one policy, two backends

One policy type, two implementations, selected at compile time.

```
struct Confine { root: PathBuf, tmp: PathBuf }
```

- Linux: build a Landlock ruleset in the parent, restrict the child in `pre_exec`.

- macOS: prefix the argument vector with `sandbox-exec -p '<profile>'`.

The alternative was one container per session, entered per call. That is a single mechanism on both platforms. It was rejected because `bash` would see the container's filesystem and toolchain while `read`, `write` and `edit` still see the host: four tools, two filesystems. It also makes Docker or Podman a hard dependency, and a Linux virtual machine on macOS.

Both are enforced by the kernel at `open`. Both are inherited by every descendant process. Neither can be lifted by the process it applies to. Both resolve symlinks and `..` themselves, so they have no time-of-check-to-time-of-use window. A path guard in minima's own process does have one.

### The intersection policy

Express only what both systems enforce, so behaviour matches on both platforms.

| Rule | Landlock | Seatbelt | In policy |
|-|-|-|-|
| Deny write outside the root | yes | yes | yes |
| Allow read everywhere | yes | yes | yes |
| Allow write to the temp directory and `/dev/null` | yes | yes | yes |
| Deny network | TCP bind and connect only, ABI 4+ | all protocols | no |
| Deny execution of named programs | yes | yes | no, every command needs `exec` |

The temp directory is not optional. On macOS `TMPDIR` is a per-user path under `/var/folders`, not `/tmp`. Omitting it breaks most toolchains.

Network is the one rule with no common ground. Leave it out, or ship it as a separate flag documented as asymmetric. Folding it into one flag would make that flag mean different things on the two platforms.

### Linux: Landlock

Landlock is a Linux security module. An unprivileged process applies a filesystem ruleset to itself, and the ruleset is inherited across `fork` and `exec` ([kernel documentation](https://docs.kernel.org/userspace-api/landlock.html)).

Measured on this machine, 2026-09-19:

- kernel `7.0.0-31-generic`

- `/sys/kernel/security/lsm` reads `lockdown,capability,landlock,yama,apparmor,ima,evm`

- `landlock_create_ruleset(NULL, 0, LANDLOCK_CREATE_RULESET_VERSION)` returns ABI **8**

ABI 8 is well past the versions that matter: 1 (5.13) filesystem, 3 (6.2) truncate, 4 (6.7) TCP.

Two steps, and the split between them is load-bearing:

1. In the parent, before `spawn`: create the ruleset, add one rule per allowed path, keep the ruleset file descriptor.

2. In `pre_exec`: call `prctl(PR_SET_NO_NEW_PRIVS, 1)` and `landlock_restrict_self(fd)`. Nothing else.

`pre_exec` runs between `fork` and `exec` in a process with a running tokio runtime. Only async-signal-safe work is allowed there. Allocating or taking a lock can deadlock against a mutex held by another thread at the moment of the fork. Building the ruleset in the parent leaves two bare system calls in the child.

The `libc` crate carries no Landlock bindings. Checked against the version in the tree: no `SYS_landlock_*` constants and no `landlock_ruleset_attr`. The options are the [`landlock` crate](https://crates.io/crates/landlock), maintained by the feature's author, or `syscall()` with hardcoded numbers. Hardcoding saves a dependency and costs the ABI negotiation that the crate performs.

### macOS: Seatbelt

Seatbelt is the macOS sandbox. `/usr/bin/sandbox-exec` applies a profile to a command and its descendants. Profiles are written in SBPL, a Scheme-like language, and `-p` takes one inline, so no temporary file is needed.

Status: Apple deprecated the interface, it remains present and functional in current macOS, and Apple's own components still use it. Third-party tools including Claude Code's sandbox runtime depend on it. Apple has not published a removal timeline ([apple/containerization#737](https://github.com/apple/containerization/issues/737)). One known limitation: SBPL rejects `setenv` and `unsetenv` operations, which has broken other projects' generated profiles ([opencode-swarm#2590](https://github.com/ZaxbyHub/opencode-swarm/issues/2590)).

The shape:

```
sandbox-exec -p '(version 1)(allow default)(deny file-write*)
                 (allow file-write* (subpath "<root>") (subpath "<tmp>"))' bash -c '<command>'
```

### Where it hooks in minima

One spawn site, `src/tools/bash.rs:86-98`. `tokio::process::Command::as_std_mut` (`tokio-1.53.1/src/process/mod.rs:329`) returns `&mut std::process::Command`, which carries `CommandExt::pre_exec`. The macOS side changes the argument vector only.

Nothing else changes. `process_group(0)` still makes `bash` a group leader, so the timeout and cancel paths still reach the whole group. The login-shell refusal is unaffected, because the command text does not change.

The file tools keep their userspace check. minima's own process must stay unconfined; it reads the config directory, the model cache and the prompt history. One policy, two places it is enforced.

## What it does not fix

- Network. Not in the intersection policy, so `curl` still reaches the network from `bash`. An adversarial model can still exfiltrate whatever it can read.

- Reading. The policy allows reads everywhere, to keep the user's headers and toolchain visible.

- The README warning. A container is still required for untrusted prompts. The warning does not shrink; it gains a platform table.

The policy is also wider than the root. Toolchains keep state outside any project. Measured on one
developer machine, 2026-09-19: `~/.cargo/registry` 2.2G, `~/.cache` 7.9G, `~/.rustup` 2.6G,
`~/.local/share` 1.8G, `~/go/pkg` 873M, `~/.npm` 140M. A ruleset allowing only the root breaks the
first build that fetches a dependency, so it must also allow `$TMPDIR` and the per-ecosystem
caches. The sandbox would prevent `rm -rf ~/documents` and permit `rm -rf ~/.cargo`.

Measured and worth recording: a `cargo build` with dependencies already fetched wrote 0 files under
`~/.cargo`. Writes outside the project happen on dependency fetch, not on every build.

## Cost

Estimated, not measured:

- Linux backend: 110-140 lines, plus the `landlock` crate or hardcoded syscall numbers.

- macOS backend: 40-60 lines.

- Shared policy type, flag handling, degradation: 40-60 lines.

- Total: roughly 200-260 lines and one dependency.

For scale, `src/tools/bash.rs` is 446 lines and the whole of `src/` is about 5,500.

## Open decisions

1. **Degradation.** The ruleset can fail: an old kernel, Landlock absent from the boot `lsm=` list, `sandbox-exec` removed. Failing quietly is the worst outcome, because the user believes there is a boundary and there is not. Anything shipped here must refuse to start rather than continue unconfined.

2. **Network.** Out of the intersection. Leave it, or accept a documented asymmetry.

Reads need no decision. The policy above does not bound them.

## Current implementation

`--root` combines the path guard and the platform sandbox. The path guard covers structured file
tools. Landlock or Seatbelt covers `bash` and its descendants. Network remains available, so this
is a filesystem boundary rather than a complete containment mechanism.

The following records why the original sandbox-only proposal was rejected:

1. **It does not retire the warning.** Network stays open, so an adversarial model still exfiltrates. A container remains necessary for untrusted prompts. The README gains a platform table and loses nothing.

2. **The macOS half cannot be tested here.** There is no CI that runs the test suite; `.github/workflows/release.yml` is tag-triggered and only builds. Its `macos-15` runner never runs `cargo test`. A Seatbelt profile would ship unverified. Untested code that claims a security boundary is worse than no code.

3. **sanduk already does this.** README.md:7 names it as the sibling project that runs minima in an Apple container or a Docker container. A container covers the network as well, which this design does not.

4. **It contradicts the project's thesis.** The README states minima "tests how small a usable agent harness can be when the ecosystem carries its capabilities". Sandboxing is such a capability, and the sibling project carries it.

The strongest argument on the other side, recorded so it is not lost: with no `rm` or `mv` tool,
every destructive operation already reaches `bash`, so a kernel policy is the only thing that could
prevent `rm -rf` outside the root. It is not enough, given the caches above and reason 2.

### What would reverse this

- Users report `bash` accidents outside the working directory. The gap above says they are
  possible; nothing here says how often they happen.

- A CI job runs the test suite on `macos-15`, so the Seatbelt profile can be verified.

- Network denial becomes expressible on both platforms, at which point the README warning could actually be retired rather than extended.
