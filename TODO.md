# TODO

## Critical

## High

## Medium

- [ ] **Session resume**. Design in `docs/dev/session-resume.md`; four decisions marked Open there. Note `src/cache.rs` has an XDG path, a schema version, an endpoint-keyed filename and a tmp-plus-rename atomic write, and `config::restrict_to_owner` already sets 0700/0600 for the history file. Persistence is open; this widens what is stored, not whether anything is. (Estimated 120-180 lines.)

- [ ] **The Linux half of the writable-set audit.** `docs/dev/root-sandbox.md` records that on macOS no cache entry is needed for a build to succeed, only for a fetch; go is an exception, remeasured 2026-10-04. The original claim that a denied `$CARGO_HOME` breaks the build was measured on Linux under Landlock and is untested against that question; CI runs `ubuntu-24.04`, so it can settle it. Trigger the `sandbox-linux` workflow and record the result in `root-sandbox.md`.

## Low

- [ ] **Create-without-delete for the writable set.** Landlock has separate `MakeReg`, `WriteFile`, `RemoveFile` and `Truncate` bits and the ruleset grants `AccessFs::from_all`; SBPL has `file-write-create` and `file-write-data` apart from `file-write-unlink`. A package store the agent can add to but not delete from would match the accident model exactly. Risk is a half-written package with no way to clean it up. See `docs/dev/root-sandbox.md`.

