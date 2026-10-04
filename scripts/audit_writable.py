#!/usr/bin/env python3
"""Toolchain workloads under `minima --sandbox` with their cache outside the write set.

    cargo build && scripts/audit_writable.py
    SANDBOX=off scripts/audit_writable.py    # control: every workload should succeed

Repeats the macOS table in docs/dev/root-sandbox.md, "The writable set", on the host platform.
minima starts with `CARGO_HOME`, `GOPATH` and `XDG_CACHE_HOME` pointing at paths that do not
exist, so the policy grants none of the real caches. Each command then points its tool at a cache
under $HOME, outside the root and $TMPDIR. Outcomes are reported against the macOS column, not
judged; only the controls fail the run.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.environ.get("BIN", os.path.join(ROOT, "target", "debug", "minima"))
SANDBOX = os.environ.get("SANDBOX", "on") != "off"

# Under $HOME, not $TMPDIR, so that no cache here is writable by the temp-directory grant.
audit = tempfile.mkdtemp(prefix="minima-audit-", dir=os.path.expanduser("~"))
root = os.path.join(audit, "root")
decoy = os.path.join(audit, "decoy")
script = os.path.join(audit, "script.json")
A = audit


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w") as f:
        f.write(text)


def have(tool):
    return shutil.which(tool) is not None


CRATE = '[package]\nname = "{0}"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\nitoa = "1"\n'
for name in ("c1", "c2"):
    write(f"{root}/{name}/Cargo.toml", CRATE.format(name))
    write(f"{root}/{name}/src/main.rs", "fn main() { let _ = itoa::Buffer::new(); }\n")
for name in ("g0", "g1", "g2"):
    write(f"{root}/{name}/go.mod", f"module {name}\n")
    write(f"{root}/{name}/main.go", "package main\n\nfunc main() {}\n")
write(f"{root}/n/package.json", '{"name": "n", "scripts": {"hello": "echo HELLO"}}\n')
for name in ("cargo-fresh", "npm", "uv", "probe"):
    os.makedirs(os.path.join(A, name))
os.makedirs(decoy)

# Populated unconfined, so the offline build has its dependency in the registry.
if have("cargo"):
    subprocess.run(["cargo", "fetch", "-q"], cwd=f"{root}/c1", check=True,
                   env=dict(os.environ, CARGO_HOME=f"{A}/cargo"))
# Initialised unconfined from another package, so `g1` is still a miss.
if have("go"):
    subprocess.run(["go", "build", "-o", "x", "."], cwd=f"{root}/g0", check=True,
                   env=dict(os.environ, GOCACHE=f"{A}/gocache", GOPATH=f"{A}/gopath",
                            GOMODCACHE=f"{A}/gopath/pkg/mod", GOTOOLCHAIN="local", GOFLAGS=""))

GO = f"GOPATH={A}/gopath GOMODCACHE={A}/gopath/pkg/mod GOTOOLCHAIN=local GOFLAGS="
# (name, required tool, command, macOS result; None for a control)
cases = [
    ("control: relocated cache denied", None, f"touch {A}/probe/x", None),
    ("cargo build --offline, registry populated", "cargo",
     f"cd c1 && CARGO_HOME={A}/cargo cargo build --offline -q", "succeeds"),
    ("cargo build, populating a fresh CARGO_HOME", "cargo",
     f"cd c2 && CARGO_HOME={A}/cargo-fresh cargo build -q", "fails"),
    # The table recorded success; remeasured 2026-10-04 with go 1.27.1, it fails on the first
    # new cache entry. Only a build whose every action hits succeeds.
    ("go build, GOCACHE exists, package not cached", "go",
     f"cd g1 && {GO} GOCACHE={A}/gocache go build -o x .", "fails"),
    ("go build, GOCACHE does not exist", "go",
     f"cd g2 && {GO} GOCACHE={A}/gocache-missing go build -o x .", "fails"),
    ("npm install, no dependencies, + npm run", "npm",
     f"cd n && export npm_config_cache={A}/npm npm_config_update_notifier=false"
     " && npm install -s --no-audit --no-fund && npm run -s hello", "succeeds"),
    ("uv venv", "uv", f"UV_CACHE_DIR={A}/uv uv venv -q v", "fails"),
    ("R -e '1+1'", "R", "R -q -e '1+1'", "succeeds"),
]
cases = [c for c in cases if c[1] is None or have(c[1])]
skipped = sorted({t for t in ("cargo", "go", "npm", "uv", "R") if not have(t)})

usage = {"usage": {"prompt_tokens": 1, "completion_tokens": 1}}
turns = [
    [{"tool_call": {"index": 0, "id": f"c{i}", "name": "bash", "arguments": json.dumps(
        {"command": f"{command} && echo AUDIT-OK", "timeout_ms": 600000})}}, usage]
    for i, (_, _, command, _) in enumerate(cases)
]
turns.append([{"text": "done\n"}, usage])
write(script, json.dumps(turns))

env = dict(os.environ, CARGO_HOME=f"{decoy}/cargo", GOPATH=f"{decoy}/go",
           XDG_CACHE_HOME=f"{decoy}/cache")
env.pop("GOMODCACHE", None)
failed = 0
try:
    run = subprocess.run(
        [BIN, "--mock", script, "--context", "1000000", "--max-turns", "64",
         *(["--sandbox"] if SANDBOX else []), "-p", "go", "--json"],
        cwd=root, env=env, capture_output=True, text=True, timeout=1800,
    )
    records = [json.loads(line) for line in run.stdout.splitlines() if line.startswith("{")]
    results = [r for r in records if r["type"] == "tool_result"]
    print(f"{sys.platform}: minima exit {run.returncode}; sandbox={SANDBOX}"
          + (f"; skipped, not installed: {', '.join(skipped)}" if skipped else ""))
    if run.stderr.strip():
        print("stderr:", run.stderr.strip()[:400])
    for (name, _, _, macos), result in zip(cases, results):
        got = "succeeds" if "AUDIT-OK" in result["output"] else "fails"
        if macos is None:
            ok = got == ("fails" if SANDBOX else "succeeds")
            failed += not ok
            label = "PASS " if ok else "FAIL "
        else:
            label = "MATCH " if got == macos else "DIFFER"
        print(f"{label}  {name:44} {got:9} (macOS: {macos or '-'})")
        if got == "fails" or label == "DIFFER":
            tail = result["output"].strip().splitlines()[-4:]
            print("\n".join(f"        {line[:200]}" for line in tail))
    if len(results) != len(cases):
        print(f"FAIL   only {len(results)} of {len(cases)} calls ran")
        failed += 1
finally:
    shutil.rmtree(audit, ignore_errors=True)

sys.exit(1 if failed else 0)
