# Confining the bash tool

Whether `bash` can be confined to one directory, and whether a path guard on the other tools is worth having without it. Written 2026-09-19 against minima 0.3.0, which rejected both. Reversed 2026-09-20: both shipped. The rejection reasoning is kept below, because three of its four points still hold and only the fourth was answered.

`--root DIR` defaults to the working directory. `--sandbox`, off by default, enforces it: Landlock on Linux and Seatbelt on macOS on `bash`, and a path check on `write` and `edit`. Until 2026-09-22 this was `--confine none|paths|fs` with `paths` the default; the middle mode bounded the file tools but not `bash`, and was dropped as misleading. Sections below written before then use the old names.

## The problem

A path guard in minima's own process can refuse `write` and `edit` outside one directory. `bash` ignores it, because a shell can run any program on the machine.

Two threat models:

1. A model that errs. It writes to the wrong path by mistake. A path guard refuses this for `write` and `edit`.

2. A model that is adversarial, or a prompt injection. It intends to leave the directory. A path guard refuses nothing, because `bash` is one call away.

Only an operating system facility closes case 2 for `bash`.

## Why a path guard on the other tools is not worth having alone

minima has four tools (`src/tools/mod.rs`), and the freeze at four is what buys the enum over `dyn Tool`. None of them renames, deletes or creates a directory. So `rm`, `mv`, `mkdir`, `git checkout` and `git apply` can only go through `bash`. That is forced by the tool set, not a model habit.

A guard on `write` and `edit` therefore prevents a file being created in the wrong place. It does not prevent `rm -rf ~/notes`. The destructive operations are exactly the ones it cannot see.

Constraining what the model generates does not help either, because the constraint is semantic rather than syntactic. Where a path resolves is a runtime fact.

- Prompt and tool-description steering is probabilistic. Measured in this project on an easier rule: GPT models wrapped commands in `bash -lc` despite the tool description, in 2 of 7 live runs of `gpt-5-nano` (CHANGELOG, 0.3.0).

- Constrained decoding needs logit access, which no provider API exposes. A grammar constrains syntax; it cannot express "this path resolves inside the root".

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

The policy is also wider than the root. Toolchains keep state outside any project. Measured on one developer machine, 2026-09-19: `~/.cargo/registry` 2.2G, `~/.cache` 7.9G, `~/.rustup` 2.6G, `~/.local/share` 1.8G, `~/go/pkg` 873M, `~/.npm` 140M. A ruleset allowing only the root breaks the first build that fetches a dependency, so it must also allow `$TMPDIR` and the per-ecosystem caches. The sandbox would prevent `rm -rf ~/documents` and permit `rm -rf ~/.cargo`.

Measured and worth recording: a `cargo build` with dependencies already fetched wrote 0 files under `~/.cargo`. Writes outside the project happen on dependency fetch, not on every build.

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

Shipped 2026-09-20. The intersection policy above, unchanged: reads everywhere, writes under the root, `$TMPDIR`, `/dev/null` and the ecosystem caches. `write` and `edit` keep the userspace check, bounded by the root alone, because nothing needs them to reach a cache. `read` has no check, since `bash` reads everything regardless. Network stays open, so this is a filesystem boundary and not containment.

Landlock is pinned to ABI 3 as a hard requirement, which means kernel 6.2. ABI 1 denies every rename across directories, breaking `mv` inside the root; ABI 2 leaves `Truncate` unhandled, so the read grant on `/` would still permit `: > file` anywhere, which destroys a file without writing to it. `IoctlDev` arrives in ABI 5 and is not handled, so ioctls on device files the command can open are unrestricted. Debian 12, RHEL 9 and Ubuntu 22.04 GA sit below the floor, which is why the sandbox is opt-in rather than the default.

Degradation is open decision 1, answered as it was written: `tools::preflight` runs one confined command at startup and fails the run if the sandbox cannot be installed. That answer is what makes the sandbox opt-in. A mode that refuses to start cannot be the default when the floor excludes Debian 12, RHEL 9 and Ubuntu 22.04 GA, and the alternative -- starting unconfined with a warning -- is the quiet failure the decision rejects. The flag carries the choice instead of the runtime guessing.

Measured 2026-09-20, correcting the measurement below: an offline `cargo build` opens
`$CARGO_HOME/.package-cache`, `.global-cache` and `.package-cache-mutate` with `O_RDWR|O_CREAT` on
every invocation. The earlier note that a build "wrote 0 files" under `~/.cargo` counted content writes, not opens. Cache writes are therefore required for any build, not only for a dependency fetch, which is what moved the caches onto the permissive side of the policy.

## Protected paths

Added 2026-09-20, after the sandbox. `write` and `edit` refuse a resolved path under the root with a `.git` component, or a component beginning `.env`. The model used was [pi's protected-paths extension](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/examples/extensions/protected-paths.ts),
which blocks its `write` and `edit` on `path.includes(".env" | ".git/" | "node_modules/")`.

Three changes to that rule. It matches the path after resolution, so `../elsewhere/.env` is caught and a root reached through a symlink still matches. It matches whole components, so `.github/`, `.gitignore` and `env.sample` are not caught, and `.env` matches as a prefix, so `.env.production` is.

The prefix then exempts `.envrc` and any name containing `example`, `sample` or `template`. Those are committed files holding no secret, and they are the `.env`-family file an agent edits most often, when a feature adds a config variable. Elsewhere in the family the exemption is not worth making: over-blocking `.env.test` costs a turn, while a missed `.env.production` costs a secret the repository cannot restore. `.git` stays whole. Sparing `hooks/` and `info/exclude`, which have uses no `git` subcommand covers, means enumerating the irreplaceable parts instead, which is more rule than a guard `bash` ignores can carry.

A refusal costs the model one turn, not the capability: it reaches the file through `bash` on the next call. It also moves the write to a worse mechanism, since `write` and `edit` replace a file by rename and `printf >` truncates in place. That is the argument for keeping the list short.

The question is whether the kernel policy can carry the same rule, since `bash` is where every deletion happens.

- **Seatbelt: yes.** SBPL takes the last matching rule, so a deny appended after the root allow holds. Measured 2026-09-20 on macOS 25.6 against a temp tree: with `(allow file-write* (subpath "<root>")) (deny file-write* (subpath "<root>/.git"))`, a write under `<root>/sub` returns 0, a write to `<root>/.git/config` and an `rm` of it both fail with `Operation not permitted`, and `cat` of it still works.

- **Landlock: no.** "One policy layer grants access to a file path if at least one of its rules encountered on the path grants the access" ([kernel documentation](https://docs.kernel.org/userspace-api/landlock.html)). Rules met walking a path are unioned, so `PathBeneath(root, write)` grants write to everything below the root and a nested rule carrying fewer rights subtracts nothing. There is no deny rule, and a second stacked layer has the same shape. The only expressible form is to drop the root rule and enumerate the root's children minus the protected ones, which leaves no rule on the root directory itself and so denies `MakeReg` and `MakeDir` there: the model could no longer create a file at the top level of the project.

So the rule is enforced in minima's process only, on both platforms, and the README says `bash` ignores it. Shipping the Seatbelt half alone was rejected on the intersection principle above, and on a worse failure it invites: a boundary tested on a Mac and trusted on Linux.

This also settles the same question for reads. Denying a read of `.env` would need a hole in the rule granting reads on `/`, which is the identical Landlock limit.

## The writable set, and why it cannot be complete

The policy permits writes to five paths outside the root. The bar they were chosen against was "a build that fetches nothing still opens it". Measured against that bar on macOS 25.6, 2026-09-20, the bar turns out to be the wrong one: no cache entry is required for a build to succeed.

Method: the SBPL profile `sandbox_command` builds, with one candidate path removed at a time, over a temp project as the root.

| Case | Cache denied | Result |
|-|-|-|
| `cargo build --offline`, from clean, dependency already in the registry | `$CARGO_HOME` | succeeds |
| `cargo build`, populating a fresh `CARGO_HOME` | `$CARGO_HOME` | fails: `failed to create directory .../registry/cache` |
| `go build`, `GOCACHE` exists, forced cache miss | `~/Library/Caches` | succeeds, silently without caching |
| `go build`, `GOCACHE` does not exist | `~/Library/Caches` | fails: `failed to initialize build cache` |
| `npm run`, and `npm install` with no dependencies | `~/.npm` | succeeds |
| `uv venv` | `~/.cache` | fails: `Failed to initialize cache at ~/.cache/uv` |
| `R -e '1+1'` | everything | succeeds |

So the entries earn their place for a different reason than the one recorded: they are there so the agent can **fetch a dependency**, not so a build can run. A denied cache costs caching and blocks `cargo add`, `go get` and `npm install`; it does not break compilation. `uv` is the exception, and it is already covered by the `$XDG_CACHE_HOME` entry.

This also narrows what `~/Library/Caches` is for. Not `uv`, which prefers `~/.cache` when it exists, and not Homebrew, whose prefix is outside the root and stays denied either way. It is there because Go's build cache lives under it on macOS, and because `$XDG_CACHE_HOME` was the Linux half of a rule written on Linux.

The doc's earlier claim -- that an offline `cargo build` opening `$CARGO_HOME/.package-cache` means a policy without the caches denies the build -- conflates the open happening with the build failing. The open may well happen; cargo tolerates its failure. That was measured on Linux under Landlock and is untested against this question there, so CI is what would settle whether Landlock behaves as Seatbelt does here.

That bar cannot cover every ecosystem, because the paths divide into two kinds and only the first belongs in a policy at all.

| Kind | Examples | Loss if destroyed |
|-|-|-|
| Regenerable cache | `$CARGO_HOME/.package-cache`, `$GOCACHE`, `~/.npm/_cacache`, `~/Library/Caches`, `~/.cabal/packages` | a re-download |
| Installed environment | `~/R/library`, `~/.opam/<switch>`, `~/.stack`, `~/.ghcup`, `~/.rustup`, `~/.cabal/store` | hours of rebuilds, shared with every other project on the machine |

Writing to the second kind is the global environment change the policy exists to prevent: installing an R package into the user library changes every other R project on the machine. Each of those ecosystems has a project-local form -- `renv` or `R_LIBS_USER`, `opam switch create .` giving `./_opam`, a relocated cabal store or stack root -- which puts the store inside the root, where it is already writable and where a reproducible project wants it. `AGENTS.md` is the place to tell the model a project works that way.

`--writable DIR` covers what is left. It is the escape hatch that makes an incomplete built-in set safe to ship: the person who uses OCaml knows they need `~/.opam`, and minima does not have to know. Waiting for a complete list is what kept this work unmerged, and there is no complete list.

Not implemented, and worth testing before it is: both backends can express create-and-write without delete-or-truncate. Landlock has separate `MakeReg`, `WriteFile`, `RemoveFile` and `Truncate` bits, and the ruleset here grants `AccessFs::from_all`, which is all of them; SBPL has `file-write-create` and `file-write-data` apart from `file-write-unlink`. A store the agent can add to but not delete from would match the accident model exactly. The risk is a half-written package with no way to clean it up, which may be worse than a denial.

## Why it was rejected first

Recorded as written on 2026-09-19. Reason 2 was answered: `.github/workflows/ci.yml` runs the suite on `ubuntu-24.04` and `macos-15`, so the Seatbelt profile is tested. Reasons 1, 3 and 4 still hold, and the feature was taken anyway on a narrower claim: sanduk contains an adversarial prompt, and this contains an accident. They are different products.

1. **It does not retire the warning.** Network stays open, so an adversarial model still exfiltrates. A container remains necessary for untrusted prompts. The README gains a platform table and loses nothing.

2. **The macOS half cannot be tested here.** There is no CI that runs the test suite; `.github/workflows/release.yml` is tag-triggered and only builds. Its `macos-15` runner never runs `cargo test`. A Seatbelt profile would ship unverified. Untested code that claims a security boundary is worse than no code.

3. **sanduk already does this.** README.md:7 names it as the sibling project that runs minima in an Apple container or a Docker container. A container covers the network as well, which this design does not.

4. **It contradicts the project's thesis.** The README states minima "tests how small a usable agent harness can be when the ecosystem carries its capabilities". Sandboxing is such a capability, and the sibling project carries it.

The strongest argument on the other side, recorded so it is not lost: with no `rm` or `mv` tool, every destructive operation already reaches `bash`, so a kernel policy is the only thing that could prevent `rm -rf` outside the root. It is not enough, given the caches above and reason 2.

## Writes the file rules do not see (macOS)

Measured 2026-09-21 on macOS 26.7 with `scripts/test_sandbox_macos.py`, against the allow-default profile that denied only `file-write*`:

| Command | Result | Why |
|-|-|-|
| `defaults write` | escaped | cfprefsd writes the plist, outside the sandbox |
| `kill` of a user process outside | escaped | a signal is not a file write |
| `launchctl submit` | denied | launchd refuses a sandboxed caller, exit 1 |
| `security add-generic-password` | denied | securityd returns `EPERM` |
| proc macro under `RUSTC_WRAPPER=sccache`, incremental | denied | sccache does not cache it; rustc ran in the client |
| the same, `CARGO_INCREMENTAL=0` | escaped, now denied | a cacheable compile ran in the sccache server, started outside |
| file in `$CARGO_HOME/bin` | written | all of `$CARGO_HOME` is granted |
| `launchctl bootout`, `kickstart`, `kill`, `stop`, `remove`, `unload`, `setenv` | denied | launchd refuses a sandboxed caller |
| `launchctl disable`, `enable` | escaped, still open | launchd accepts them; the record survives reboot |
| `open x.command` | denied by macOS | Terminal refuses a sandboxed caller, with a dialog; `open` still exits 0 |
| `open P.app`, built in the root | escaped, now denied | launchd starts the app outside the sandbox; `(deny lsopen)` also blocks pages and URLs |

`~/Library/Caches` held 157 entries on the measuring machine, almost all of them apps'. Measured 2026-09-22, each workload with the directory denied, then granted with the entries it wrote listed:

| Workload | Denied | Writes |
|-|-|-|
| `go build`, `go vet`, `go test` | succeeds, uncached | `go-build` |
| `pip install` | succeeds, warns the cache is not writable | `pip` |
| `/usr/bin/python3` | succeeds, no bytecode cache | `com.apple.python` |
| `swift build` | see below | `org.swift.swiftpm` |
| `ccache clang` | fails | `ccache` |
| `deno` npm import | fails | `deno` |
| `swiftc`, `bun add`, `brew info`, `Rscript` | succeeds | nothing there |

The grant is those six entries. Tools not measured -- Yarn, pnpm, CocoaPods, Playwright -- need `--writable` until they are.

`swift build` fails under `--sandbox` whenever it compiles `Package.swift`: SwiftPM runs the manifest compiler under its own `sandbox-exec`, and Seatbelt refuses `sandbox_apply` inside a sandbox. A cached manifest in `org.swift.swiftpm` hides this on a rebuild. `--disable-sandbox` works; no environment variable was found that does the same, and `SWIFTC_DISABLE_SANDBOX` does not. A command that fails this way gets a note naming `--disable-sandbox`. SwiftPM also warns that `~/Library/org.swift.swiftpm/configuration` is not accessible; that is user configuration, not a cache, and stays denied.
| `swiftc`, `clang -fmodules` | failed | module cache in `DARWIN_USER_CACHE_DIR`, beside `$TMPDIR` |

The profile now adds `(deny user-preference-write)`, `(deny signal) (allow signal (target same-sandbox))`, and the user cache directory. This departs from the intersection policy. The signal rule needs Landlock ABI 6 (`Scope::Signal`), above the ABI 3 floor. The preference rule has no Linux counterpart; dotfile config there is an ordinary file write, already denied. Both close accident-class escapes, which is the threat model, so a macOS-only rule is not a boundary Linux users would be misled into trusting.

`$CARGO_HOME/bin` was open, and is now closed. It is an installed environment by the table above, not a cache, and rustup puts it on `PATH`, so a file written there ran unconfined in the user's next shell. The grant is now `registry/`, `git/`, `.package-cache`, `.package-cache-mutate`, `.global-cache` and `.global-cache-journal`. Measured 2026-09-22: without the two lock files cargo logs `failed to acquire cache lock` and fetches unlocked; without the journal (the database is in `journal_mode=delete`) it cannot save last-use data for its garbage collector. The journal exists only during a write, which Landlock cannot grant, so on Linux the record is lost and the build still succeeds.

`$GOPATH/bin` was the same case, and is closed the same way. The grant is `pkg/mod` and `pkg/sumdb` under the first `$GOPATH` entry, plus `$GOMODCACHE` when set. Measured 2026-09-22: with `pkg/mod` alone, fetching an uncached module fails with `verifying module: open .../pkg/sumdb/sum.golang.org/latest: operation not permitted`. With both, the fetch and build succeed and `go install` is denied.

A fresh install has none of these entries, and a confined command cannot create them: go fails `mkdir .../pkg: operation not permitted`, measured 2026-09-22 on macOS. Cargo succeeded there, because a Seatbelt `subpath` on `registry/` lets cargo create it; Landlock drops the rule instead, so Linux is expected to fail too (inferred, not measured). The preflight therefore creates them, only under a `$CARGO_HOME` or `$GOPATH` that already exists.

### Build servers

A build tool that hands work to a long-lived server runs that work with the server's bounds. Measured with sccache, which `RUSTC_WRAPPER` often names:

- Server started outside: every cacheable compile, which is every registry dependency, runs unconfined, proc macros and all.

- Server started by a confined build: it keeps that build's policy after minima exits. Every later cache hit, confined or not, then fails with `error writing dependencies ... Operation not permitted`, because the root it was allowed to write no longer matches. Stopping the server is the fix.

`--sandbox` now sets `RUSTC_WRAPPER` and `RUSTC_WORKSPACE_WRAPPER` empty. Measured 2026-09-22: empty overrides `build.rustc-wrapper` and `build.rustc-workspace-wrapper` in cargo config, and unset does not. Denying loopback connections in SBPL was the alternative: it covers every server, but only on macOS, and it breaks the local dev server an agent starts to test against.

This bounds a mistake, not a model that means to escape: a command can set the wrapper again or connect to any server itself. Gradle's daemon and Bazel's server have the same shape and are not handled.

## Merge readiness: `sandbox` into `main`

Assessed 2026-09-22 at `31d6ea4`, updated at `935b4f9` for the switch from `--confine` to `--sandbox`. Recommendation: merge. `ci` passes on `935b4f9` (run 35744601974), with `scripts/test_sandbox.py` at 22/22 on both runners. Do not tag a release from the merge without the items under "Before releasing".

### State of the branch

- 12 commits ahead of `main`, 0 behind. The merge is a fast-forward; `git merge-tree` reports no conflicts.

- 18 files, +2,170 / -118. `src/` is +1,344 / -65 across 8 files, tests included. `src/tools/bash.rs` went from 519 lines to 674 non-test and 613 test lines. The estimate under "Cost" was 200-260 lines; the difference is the writable-set work, the macOS rules and the notes.

- One new dependency, Linux only: `landlock = "0.4.7"`.

- CI at `b4d43d7`: `ci` passes lint and test on `ubuntu-24.04` and `macos-15`, including `scripts/test_sandbox.py` at 19/19 on both. `sandbox-macos` passes `scripts/test_sandbox_macos.py` at 25/25 plus 3 known. `31d6ea4` only removes that workflow's temporary push trigger; `ci` passes there too (run 35734868981).

### What merging changes for users

- **Nothing, by default.** The sandbox is off unless `--sandbox` is passed, and off bounds nothing,
  as on `main`. `scripts/test_sandbox.py` with `SANDBOX=off` confirms it: every escaping write
  lands, the `write` tool's included.
- `--sandbox`, `--writable` and `--root` are opt-in.
- The `--json` result record gains `sandbox` and `writable`. Additive; a strict schema consumer
  would see new fields.

### For merging

- Both backends run in CI on every push, and the macOS end-to-end script has a workflow. The rejection's reason 2, "the macOS half cannot be tested here", no longer holds.

- Every escape found on macOS is either closed and covered by a test, or recorded below as known. Each closing rule was checked by removing it and watching its test fail.

- `--sandbox` fails at startup when the platform cannot install the sandbox, so no user is told there is a boundary when there is none.

- The branch is behind nothing, so it will not grow harder to merge; it will only diverge further from `main` while it waits.

### Against merging now

- **Linux is less measured than macOS.** The writable-set tables in this document were measured on macOS. On Linux, CI now measures the fresh-store fetches and the last-use loss, below, but not the per-cache denial table. The `TODO.md` entry on the Linux audit is still open.

- **The macOS policy is no longer the intersection policy.** Preference writes, signals and `open` are denied on macOS only. The rule this document set was that a boundary holding on one platform would be trusted on the other. The departure is argued in "Writes the file rules do not see", but it is a departure.

- **`--sandbox` has known costs on macOS:** `swift build` needs `--disable-sandbox`, minima's own suite cannot run confined, `open` is unavailable to the agent, and confined builds lose sccache.

- **Known gaps remain:** `launchctl disable` and `enable` (reproduced on the CI runner), build servers other than sccache, `$GOCACHE` and other relocated caches, and the open network.

- Reasons 1, 3 and 4 under "Why it was rejected first" still hold. They argued against building the feature; merging it is the smaller decision.

### Before merging

Done. `ci` run 35740317418 (`70e3585`) passes lint and test on both runners, and `scripts/test_sandbox.py` passes 22/22 on each. uv is not installed on either runner, so its case is skipped there; the Linux go case is what confirms the `$XDG_CACHE_HOME` fix in CI, and the uv case confirms it on macOS locally.

Linux results, from `ci` runs 35736572539 (`5a9af12`) and 35738450862 (`e73ebef`):

- Into an empty `CARGO_HOME` the fetch succeeds with the cache lock held, and cargo's last-use record is lost. Both as predicted.

- Into an empty `GOPATH`, go fetched the module and then failed: `failed to initialize build cache at /home/runner/.cache/go-build: mkdir /home/runner/.cache: permission denied`. The runner has no `~/.cache`, and a writable path is granted only if it exists. Not a Linux-only fault: uv on macOS fails the same way with a missing `$XDG_CACHE_HOME`, reproduced locally. The preflight now creates `$XDG_CACHE_HOME` when its parent exists, and the script runs its fresh session with that directory missing, so the case no longer depends on the runner image.

Done: the default no longer changes. `--confine none|paths|fs` became `--sandbox`, off by default (2026-09-22).

### Before releasing

1. Fold the Unreleased "Fixed" entries into "Added". Every one of them fixes a feature that has not shipped, so no user has met the bug; as written they read as regressions in 0.4.0.

2. Bump the minor version: a new flag and a new field in the `--json` result, with no default changed.

3. After the merge, start `sandbox-macos` by hand once from `main`, since `workflow_dispatch` only works from the default branch.
