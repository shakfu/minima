#!/usr/bin/env python3
"""Real commands through `minima --sandbox`, checked on disk rather than as minima reports them.

    cargo build && scripts/test_sandbox.py
    SANDBOX=off scripts/test_sandbox.py    # control: the outside writes should now land

A mock script plays the model, so no key is needed. Linux needs kernel 6.2 for Landlock; macOS uses
Seatbelt. The outside target sits under $HOME, because the policy leaves $TMPDIR writable. Every
check runs after the session, so the rm and truncate cases each get their own file: on one shared
file, a truncate that lands recreates what an rm removed.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import uuid

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.environ.get("BIN", os.path.join(ROOT, "target", "debug", "minima"))
SANDBOX = os.environ.get("SANDBOX", "on") != "off"

home = os.path.expanduser("~")
tag = uuid.uuid4().hex[:8]
out = os.path.join(home, f"minima-sbx-outside-{tag}")
extra = os.path.join(home, f"minima-sbx-writable-{tag}")
# Under $HOME, not $TMPDIR, so that nothing the root holds is writable by the temp-directory grant.
root = tempfile.mkdtemp(prefix="minima-sbx-root-", dir=home)
script = os.path.join(home, f"minima-sbx-{tag}.json")
tmpfile = os.path.join(tempfile.gettempdir(), f"minima-sbx-{tag}")
os.makedirs(out)
os.makedirs(extra)
for name in ("existing", "removable"):
    with open(os.path.join(out, name), "w") as f:
        f.write("keep me\n")


def exists(path):
    return os.path.exists(path)


def rooted(path):
    return os.path.join(root, path)


# (name, tool, arguments, check on the finished session's disk state)
cases = [
    ("cargo build inside root (writes build caches)", "bash",
     {"command": "cargo new -q --vcs none demo && cd demo && cargo build -q 2>&1 | tail -1;"
                 " test -x target/debug/demo && echo BUILT", "timeout_ms": 600000},
     lambda r: "BUILT" in r["output"]),
    ("git init + commit inside root", "bash",
     {"command": "git init -q g && cd g && echo a > a && git add a"
                 " && git -c user.email=t@t -c user.name=t commit -qm m && git log --oneline | wc -l"},
     lambda r: r["output"].strip().startswith("1")),
    # macOS: $TMPDIR is under /var/folders, which resolves through /private.
    ("write to $TMPDIR allowed", "bash", {"command": f"echo ok > {tmpfile} && cat {tmpfile}"},
     lambda r: exists(tmpfile)),
    ("read outside root allowed", "bash", {"command": "head -c 20 /etc/hosts >/dev/null && echo READ"},
     lambda r: "READ" in r["output"]),
    ("redirect into outside dir denied", "bash", {"command": f"echo x > {out}/new"},
     lambda r: not exists(f"{out}/new")),
    ("cd .. escape denied", "bash",
     {"command": f"cd {out}/.. && touch {os.path.basename(out)}/cd-escape"},
     lambda r: not exists(f"{out}/cd-escape")),
    ("symlink escape denied", "bash", {"command": f"ln -s {out} link && echo x > link/via-link"},
     lambda r: not exists(f"{out}/via-link")),
    ("python open() outside denied", "bash",
     {"command": f"python3 -c \"open('{out}/py','w').write('x')\""},
     lambda r: not exists(f"{out}/py")),
    ("rm outside file denied", "bash", {"command": f"rm -f {out}/removable"},
     lambda r: exists(f"{out}/removable")),
    ("truncate outside file denied", "bash", {"command": f": > {out}/existing"},
     lambda r: exists(f"{out}/existing") and open(f"{out}/existing").read() == "keep me\n"),
    # Judged after the session: the job leaves a marker in $TMPDIR, so a pass means it ran and was
    # denied, not that minima killed it first.
    ("background job starts", "bash",
     {"command": f"(sleep 1; echo x > {out}/bg; echo ran > {tmpfile}.bg) >/dev/null 2>&1 &"},
     lambda r: True),
    ("  (wait for the job)", "bash", {"command": "sleep 2"}, lambda r: True),
    ("--writable dir allowed", "bash", {"command": f"echo x > {extra}/granted"},
     lambda r: exists(f"{extra}/granted")),
    ("write tool outside root refused", "write", {"path": f"{out}/w", "content": "x"},
     lambda r: not exists(f"{out}/w")),
    ("write tool into .git refused", "write", {"path": "g/.git/config", "content": "x"},
     lambda r: open(rooted("g/.git/config")).read() != "x"),
    ("write tool to .env refused", "write", {"path": ".env", "content": "SECRET=1"},
     lambda r: not exists(rooted(".env"))),
    ("write tool to .env.example allowed", "write", {"path": ".env.example", "content": "SECRET="},
     lambda r: exists(rooted(".env.example"))),
    ("write tool --writable dir still refused", "write", {"path": f"{extra}/w", "content": "x"},
     lambda r: not exists(f"{extra}/w")),
]

# A second session under `--sandbox`, with a `CARGO_HOME` and `GOPATH` that exist but hold nothing and an
# `XDG_CACHE_HOME` that does not exist, as on a new machine. The preflight must create the cache
# entries, since a confined command cannot.
# Whether cargo kept its last-use record is reported, not judged: Landlock cannot grant the journal
# sqlite creates beside `.global-cache`, so on Linux the record is expected to be lost.
fresh = tempfile.mkdtemp(prefix="minima-sbx-fresh-", dir=home)
fresh_root = os.path.join(fresh, "root")
for name in ("root", "cargo", "gopath"):
    os.makedirs(os.path.join(fresh, name))
has_go = shutil.which("go") is not None
has_uv = shutil.which("uv") is not None
fresh_cases = [
    ("fresh CARGO_HOME: cargo add + build, locked", "bash",
     {"command": "cargo new -q --vcs none f && cd f && cargo add -q itoa && CARGO_LOG="
                 "cargo::util::cache_lock=warn,cargo::core::global_cache_tracker=warn cargo build -q"
                 " 2>&1 | grep WARN; test -x target/debug/f && echo BUILT", "timeout_ms": 600000},
     lambda r: "BUILT" in r["output"] and "failed to acquire cache lock" not in r["output"]),
    ("fresh GOPATH: go mod tidy + build", "bash",
     {"command": "mkdir g && cd g && go mod init x >/dev/null 2>&1 && printf 'package main\\n"
                 "import _ \"github.com/google/uuid\"\\nfunc main(){}\\n' > main.go"
                 " && go mod tidy && go build -o x . && echo BUILT" if has_go else "echo BUILT",
      "timeout_ms": 600000},
     lambda r: "BUILT" in r["output"]),
    ("fresh XDG_CACHE_HOME: uv venv", "bash",
     {"command": "uv venv -q v && echo MADE" if has_uv else "echo MADE", "timeout_ms": 600000},
     lambda r: "MADE" in r["output"]),
]


def session(case_list, cwd, bounds, env=None):
    """One mock-driven minima run over `case_list`; returns its tool results and final record."""
    usage = {"usage": {"prompt_tokens": 1, "completion_tokens": 1}}
    turns = [
        [{"tool_call": {"index": 0, "id": f"c{i}", "name": tool, "arguments": json.dumps(args)}},
         usage]
        for i, (_, tool, args, _) in enumerate(case_list)
    ]
    turns.append([{"text": "done\n"}, usage])
    with open(script, "w") as f:
        json.dump(turns, f)
    run = subprocess.run(
        [BIN, "--mock", script, "--context", "1000000", "--max-turns", "64", *bounds,
         "-p", "go", "--json"],
        cwd=cwd, capture_output=True, text=True, timeout=1200, env=env,
    )
    time.sleep(0.5)
    records = [json.loads(line) for line in run.stdout.splitlines() if line.startswith("{")]
    if run.stderr.strip():
        print("stderr:", run.stderr.strip()[:400])
    return [r for r in records if r["type"] == "tool_result"], (records[-1] if records else {}), run


def judge(case_list, results):
    failed = 0
    for (name, _, _, check), result in zip(case_list, results):
        ok = check(result)
        failed += not ok
        print(f"{'PASS' if ok else 'FAIL'}  {name:48} {(result.get('note') or '')[:70]}")
        if not ok:
            # The note keeps only the first stderr line, which is often progress, not the cause.
            tail = result["output"].strip().splitlines()[-8:]
            print("\n".join(f"      {line[:200]}" for line in tail))
    if len(results) != len(case_list):
        print(f"FAIL  only {len(results)} of {len(case_list)} calls ran")
        failed += 1
    return failed


bounds = ["--sandbox", "--writable", extra] if SANDBOX else []
failed = 0
total = len(cases) + 1
try:
    results, final, run = session(cases, root, bounds)
    print(f"{sys.platform}: minima exit {run.returncode}; "
          f"sandbox={final.get('sandbox')} writable={final.get('writable')}")
    failed += judge(cases, results)
    ran = exists(f"{tmpfile}.bg")
    ok = ran and not exists(f"{out}/bg")
    failed += not ok
    print(f"{'PASS' if ok else 'FAIL'}  background job ran (marker={ran}), outside write denied")

    if SANDBOX:
        env = dict(os.environ, CARGO_HOME=os.path.join(fresh, "cargo"),
                   GOPATH=os.path.join(fresh, "gopath"), GOFLAGS="-modcacherw",
                   XDG_CACHE_HOME=os.path.join(fresh, "cache"))
        env.pop("GOMODCACHE", None)
        results, _, _ = session(fresh_cases, fresh_root, ["--sandbox"], env)
        skipped = [tool for tool, has in (("go", has_go), ("uv", has_uv)) if not has]
        print("fresh stores:" + (f" (not installed, skipped: {', '.join(skipped)})" if skipped else ""))
        failed += judge(fresh_cases, results)
        total += len(fresh_cases)
        if results:
            lost = "failed to save last-use data" in results[0]["output"]
            print(f"INFO  cargo last-use record {'lost' if lost else 'saved'}")
finally:
    for path in (out, extra, root, fresh):
        shutil.rmtree(path, ignore_errors=True)
    for path in (tmpfile, f"{tmpfile}.bg", script):
        try:
            os.remove(path)
        except FileNotFoundError:
            pass

print(f"{total - failed}/{total} passed" + ("" if SANDBOX else " (control, no --sandbox)"))
sys.exit(1 if failed else 0)
