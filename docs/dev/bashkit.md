# bashkit as a bash backend

Whether [bashkit](https://github.com/everruns/bashkit) could replace or supplement minima's `bash` tool. Written 2026-09-25 against bashkit `eb5032f` (0.18.2) and minima after the move to `sanduk-sandbox`. Conclusion: no, except as an optional read-only inspection tool.

## What it is

bashkit is a bash interpreter in Rust. It runs in the host process over a virtual filesystem (VFS). All 167 commands (`grep`, `sed`, `awk`, `jq`, `tar`, `curl`, ...) are reimplemented in Rust. It never calls `fork` or `exec`.

- About 175k lines in `crates/bashkit/src`. Bindings for Python, JS, WASM and C.
- 51 releases since 2026-01-31. 280 stars. About 50k crates.io downloads in the last 90 days.
- One main author: 1718 commits, against 7 for the next human contributor.
- Filesystems: `InMemoryFs`, `OverlayFs` (copy-on-write), `MountableFs`, `ReadOnlyFs`. `RealFs`, behind the `realfs` feature, mounts a host directory, read-only by default.
- Limits on command count, loop iterations, function depth, output size, filesystem size and parser fuel.
- Network is denied by default. HTTP goes only through `curl`, `wget` and `http`, with a per-domain allowlist.
- `analyze()` lists commands and redirect targets without running the script. It is advisory: names built at runtime set `is_opaque()`.

## Comparison with `sanduk-sandbox`

| | `sanduk-sandbox` (Landlock, Seatbelt) | bashkit |
|-|-|-|
| Runs `cargo`, `make`, `pytest`, `git` | yes | no (L-PROC-003) |
| Write confinement | kernel, at `open` | writes reach only the VFS |
| Read confinement | no | yes |
| Network confinement | no | yes, allowlist |
| Platforms | Linux 6.2+, macOS | any, including WASM |
| Bash fidelity | real bash | "substantial" POSIX; gaps in `knowledge/operations/limitations.md` |

The first row decides it. minima's `bash` exists to run the project's toolchain. bashkit cannot run it. Python and TypeScript exist only as embedded subset interpreters (Monty, ZapCode), both marked experimental.

## Uses considered

- **Replace `bash`.** Rejected. The agent loses the ability to build and test.
- **Pre-filter commands with `analyze()`.** Rejected. It is a string check, which `root-sandbox.md` shows can be bypassed. bashkit's docs call it advisory.
- **Read-only inspection tool.** Viable. Mount the root with `RealFs` read-only and expose `grep`, `rg`, `find`, `jq`. This confines reads and network, which `--sandbox` does not. It would add a fifth tool, and the four-tool freeze in `src/tools/mod.rs` argues against that.

## Risks

- Divergences from GNU produce wrong output, not errors. A model will assume GNU behaviour. Examples:
  - `sed` `\<` and `\>` both compile to `\b` (L-SED-004).
  - Symlinks are stored but never followed (L-FS-001).
  - `local` scoping in nested functions is incomplete.
  - `return` value propagation is incomplete.
- The threat model (`knowledge/security/threat-model.md`, "280+ threats") is written by the author. I found no mention of an independent audit.
- The project is pre-1.0 and has one main author, so expect API breaks.

## If read and network confinement becomes a goal

Two options keep the real toolchain:

1. Add Landlock ABI 4+ TCP rules. This blocks TCP bind and connect only. Seatbelt can deny all network. Both confine reads.
2. Run in a container or microVM, bind-mounting the root at the same path. This answers the objection in `root-sandbox.md` that `bash` and the file tools would see two filesystems.

bashkit gets read and network confinement only by giving up the toolchain.

## Not verified

- bashkit's spec suite against GNU bash.
- Whether `RealFs` resists escapes in practice.
- Added dependencies and compile time.
