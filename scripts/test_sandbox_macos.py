#!/usr/bin/env python3
"""Real toolchains and IPC escapes through `minima --sandbox` on macOS, checked on disk.

    cargo build && scripts/test_sandbox_macos.py

The macOS complement to `test_sandbox.py`, which covers escapes by path. Three groups:

- `work`: builds that fetch from the network and must succeed under the policy.
- `deny`: writes and signals outside the root that must not land, including writes a daemon makes
  for the command (cfprefsd, launchd, securityd) and an app `open` would start, which no file
  rule sees.
- `known`: reported, not failed. Nested `sandbox-exec` is refused by Seatbelt, so minima's own
  suite cannot run under `--sandbox`, and `swift build` compiles a changed manifest under its own
  `sandbox-exec`, so it needs `--disable-sandbox`. `launchctl disable` and `enable` reach launchd's override database,
  which no SBPL rule tried blocks; every other `launchctl` change is refused.

A mock plays the model, so no key is needed; the network is. The `deny` cases change global state
under unique names -- a `defaults` domain, a keychain item, a launchd job, a `git config` key --
and remove it afterwards. The cargo test case builds minima from scratch and takes minutes.
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

if sys.platform != "darwin":
    sys.exit("macOS only; scripts/test_sandbox.py covers Linux")

home = os.path.expanduser("~")
tag = uuid.uuid4().hex[:8]
out = os.path.join(home, f"minima-rw-outside-{tag}")
# Under $HOME, not $TMPDIR, so that nothing the root holds is writable by the temp-directory grant.
root = tempfile.mkdtemp(prefix="minima-rw-root-", dir=home)
script = os.path.join(home, f"minima-rw-{tag}.json")
os.makedirs(out)
with open(f"{out}/existing", "w") as f:
    f.write("keep me\n")
name = f"minima.rw.{tag}"  # defaults domain, launchd label and keychain service
cargo_bin_probe = os.path.join(os.environ.get("CARGO_HOME", f"{home}/.cargo"), "bin", f"minima-rw-{tag}")
gopath = subprocess.run(["go", "env", "GOPATH"], capture_output=True, text=True).stdout.strip()
gobin = gopath.split(os.pathsep)[0] + "/bin"
npm = shutil.which("npm")
npm_global = npm and os.path.join(os.path.dirname(os.path.dirname(npm)), "lib/node_modules/left-pad")
victim = subprocess.Popen(["sleep", "600"])  # a process of the user's, outside the sandbox


def sh(cmd):
    return subprocess.run(cmd, shell=True, capture_output=True, text=True)


def content(path):
    try:
        return open(path).read()
    except OSError:
        return None


def says(word="OK"):
    return lambda r: word in r["output"]


# A proc macro that writes outside the root when it expands. Non-incremental, so a wrapper such
# as sccache would cache it and run rustc in its server; `--sandbox` clears the wrapper.
PROC_MACRO = f"""
mkdir pm && cd pm && cargo new -q --vcs none --lib mac && cargo new -q --vcs none --lib use_it
printf '[lib]\\nproc-macro = true\\n' >> mac/Cargo.toml
cat > mac/src/lib.rs <<'EOF'
use proc_macro::TokenStream;
#[proc_macro]
pub fn m(_: TokenStream) -> TokenStream {{
    let _ = std::fs::write("{out}/procmacro", "x");
    TokenStream::new()
}}
EOF
printf 'mac = {{ path = "../mac" }}\\n' >> use_it/Cargo.toml
echo 'mac::m!();' > use_it/src/lib.rs
cd use_it && CARGO_INCREMENTAL=0 cargo build -q 2>&1 | tail -3; echo "RUSTC_WRAPPER=[$RUSTC_WRAPPER]"
"""

# (kind, name, command, check on the finished session's disk state)
cases = [
    # Cargo carries on without its cache lock or last-use record, and says so only in the log.
    ("work", "cargo add (registry fetch) + build, locked",
     "cargo new -q --vcs none dep && cd dep && cargo add -q itoa && CARGO_LOG="
     "cargo::util::cache_lock=warn,cargo::core::global_cache_tracker=warn cargo build -q 2>&1"
     " | grep WARN; test -x target/debug/dep && echo OK",
     lambda r: "OK" in r["output"] and "WARN" not in r["output"]),
    ("work", "uv venv + uv pip install",
     "uv venv -q uvv && VIRTUAL_ENV=uvv uv pip install -q six"
     " && uvv/bin/python -c 'import six; print(\"OK\")'", says()),
    # pip still installs with its cache denied, and says so in a warning.
    ("work", "python venv + pip install, cached",
     "python3 -m venv pv && pv/bin/pip install -q --disable-pip-version-check six 2>&1 | tail -2"
     " && pv/bin/python -c 'import six; print(\"OK\")'",
     lambda r: "OK" in r["output"] and "not writable" not in r["output"]),
    ("work", "npm install",
     "mkdir n && cd n && npm init -y >/dev/null && npm install --silent --no-audit --no-fund left-pad"
     " && node -e 'require(\"left-pad\"); console.log(\"OK\")'", says()),
    ("work", "go mod tidy (fetch) + build",
     "mkdir gm && cd gm && go mod init x >/dev/null 2>&1 && printf 'package main\\nimport (\"fmt\"; "
     "\"github.com/google/uuid\")\\nfunc main(){ _ = uuid.New(); fmt.Println(\"OK\") }\\n' > main.go"
     " && go mod tidy 2>&1 | tail -2 && go build -o gx . && ./gx", says()),
    ("work", "clang -fmodules (user cache dir)",
     "printf '@import Foundation;\\nint main(){ puts(\"OK\"); }\\n' > m.m"
     " && clang -fmodules m.m -o m 2>&1 | tail -2 && ./m", says()),
    ("work", "swiftc (user cache dir)",
     "echo 'print(\"OK\")' > s.swift && swiftc s.swift -o s 2>&1 | tail -3 && ./s", says()),
    ("work", "swift build --disable-sandbox (fresh manifest)",
     "mkdir sp && cd sp && swift package init -q --type executable >/dev/null 2>&1"
     f" && echo '// {tag}' >> Package.swift && swift build --disable-sandbox >log 2>&1 && echo OK", says()),
    ("work", "ccache clang (Library/Caches/ccache)",
     "command -v ccache >/dev/null || { echo 'OK (no ccache)'; exit; };"
     f" printf 'int main(){{return {int(tag, 16) % 7};}}\\n' > cc.c && ccache clang -c cc.c -o cc.o && echo OK",
     says()),
    ("work", "deno npm import (Library/Caches/deno)",
     "command -v deno >/dev/null || { echo 'OK (no deno)'; exit; };"
     " deno eval \"import('npm:left-pad@1.3.0').then(() => console.log('OK'))\"", says()),
    ("work", "a command signals its own children",
     "sleep 30 & kill $! && wait $!; echo OWN=$?", says("OWN=143")),
    ("deny", "write to ~/Library/Caches itself (every app's caches)",
     f"echo x > ~/Library/Caches/minima-rw-{tag}",
     lambda r: not os.path.exists(f"{home}/Library/Caches/minima-rw-{tag}")),
    ("deny", "git config --global",
     "git config --global minima.rwprobe x",
     lambda r: sh("git config --global --get minima.rwprobe").stdout == ""),
    ("deny", "npm install -g",
     "npm install -g --silent --no-audit --no-fund left-pad",
     lambda r: not (npm_global and os.path.exists(npm_global))),
    ("deny", "chmod outside file", f"chmod 000 {out}/existing",
     lambda r: os.stat(f"{out}/existing").st_mode & 0o777 != 0),
    ("deny", "hard link from outside, write through it",
     f"ln {out}/existing hl && echo pwned > hl",
     lambda r: content(f"{out}/existing") == "keep me\n"),
    ("deny", "defaults write (cfprefsd)",
     f"defaults write {name} k -string x",
     lambda r: sh(f"defaults read {name} k").returncode != 0),
    ("deny", "launchctl submit (launchd)",
     f"launchctl submit -l {name} -- /bin/sh -c 'echo x > {out}/launchd'; sleep 2",
     lambda r: not os.path.exists(f"{out}/launchd")),
    ("deny", "security add-generic-password (securityd)",
     f"security add-generic-password -s {name} -a a -w w",
     lambda r: sh(f"security find-generic-password -s {name}").returncode != 0),
    # A windowless app the command builds; launchd would start it outside the sandbox.
    ("deny", "open an app bundle built in the root (LaunchServices)",
     "mkdir -p P.app/Contents/MacOS && printf '#!/bin/sh\\ntouch %s\\n' > P.app/Contents/MacOS/P"
     " && chmod +x P.app/Contents/MacOS/P && printf '<plist><dict><key>CFBundleExecutable</key>"
     "<string>P</string><key>LSUIElement</key><true/></dict></plist>' > P.app/Contents/Info.plist"
     " && open P.app; sleep 2" % f"{out}/opened",
     lambda r: not os.path.exists(f"{out}/opened") and "does not let a command open" in r["output"]),
    ("deny", "kill a process outside the sandbox", f"kill {victim.pid}",
     lambda r: victim.poll() is None),
    ("deny", "file in $CARGO_HOME/bin (on PATH for rustup users)",
     f"echo x > {cargo_bin_probe}", lambda r: not os.path.exists(cargo_bin_probe)),
    ("deny", "cargo install", "cargo install -q --path dep 2>&1 | tail -1",
     lambda r: not os.path.exists(os.path.join(os.path.dirname(cargo_bin_probe), "dep"))),
    ("deny", "go install (into $GOPATH/bin)",
     f"cd gm && go mod edit -module minima-rw-{tag} && go install . 2>&1 | tail -1",
     lambda r: not os.path.exists(f"{gobin}/minima-rw-{tag}")),
    ("deny", "proc macro in a cacheable compile, RUSTC_WRAPPER cleared", PROC_MACRO,
     lambda r: not os.path.exists(f"{out}/procmacro")),
    # SwiftPM compiles the manifest under its own sandbox-exec, which Seatbelt refuses when nested.
    ("known", "swift build on a fresh manifest (nested sandbox-exec)",
     f"cd sp && echo '// {tag}x' >> Package.swift && swift build >log 2>&1; echo rc=$?",
     says("rc=0")),
    ("known", "launchctl disable (persists across reboot)",
     f"launchctl disable gui/{os.getuid()}/{name}",
     lambda r: f'"{name}" => disabled' not in sh(f"launchctl print-disabled gui/{os.getuid()}").stdout),
    ("known", "cargo test of minima itself (nested sandbox-exec)",
     f"git clone -q {ROOT} clone && cd clone && cargo test 2>&1 | grep -E '^test result|sandbox_apply' | sort -u",
     lambda r: "sandbox_apply" not in r["output"] and "test result: ok" in r["output"]),
]

usage = {"usage": {"prompt_tokens": 1, "completion_tokens": 1}}
turns = [
    [{"tool_call": {"index": 0, "id": f"c{i}", "name": "bash",
                    "arguments": json.dumps({"command": cmd, "timeout_ms": 600000})}}, usage]
    for i, (_, _, cmd, _) in enumerate(cases)
]
turns.append([{"text": "done\n"}, usage])
with open(script, "w") as f:
    json.dump(turns, f)

failed = 0
try:
    run = subprocess.run(
        [BIN, "--mock", script, "--context", "1000000", "--max-turns", "64",
         "--sandbox", "-p", "go", "--json"],
        cwd=root, capture_output=True, text=True, timeout=3000,
    )
    time.sleep(1)
    records = [json.loads(line) for line in run.stdout.splitlines() if line.startswith("{")]
    results = [r for r in records if r["type"] == "tool_result"]
    print(f"darwin: minima exit {run.returncode}")
    if run.stderr.strip():
        print("stderr:", run.stderr.strip()[:400])
    for (kind, label, _, check), result in zip(cases, results):
        ok = check(result)
        if kind == "work":
            verdict = "PASS" if ok else "FAIL"
        elif kind == "deny":
            verdict = "DENIED" if ok else "ESCAPED"
        else:
            verdict = "HOLDS" if ok else "KNOWN"
        failed += kind != "known" and not ok
        print(f"{verdict:8} {kind:5}  {label}")
        if not ok or os.environ.get("VERBOSE"):
            print("         ", result["output"].strip().replace("\n", " | ")[:300])
    if len(results) != len(cases):
        print(f"FAIL  only {len(results)} of {len(cases)} calls ran")
        failed += 1
finally:
    victim.kill()
    sh(f"defaults delete {name}")
    sh(f"launchctl remove {name}")
    # Leaves an "enabled" entry in the override database; launchctl has no way to drop one.
    sh(f"launchctl enable gui/{os.getuid()}/{name}")
    sh(f"security delete-generic-password -s {name}")
    sh("git config --global --unset minima.rwprobe")
    if npm_global:
        shutil.rmtree(npm_global, ignore_errors=True)
    for path in (f"{home}/Library/Caches/minima-rw-{tag}", cargo_bin_probe, f"{gobin}/minima-rw-{tag}", script):
        try:
            os.remove(path)
        except FileNotFoundError:
            pass
    for path in (out, root):
        shutil.rmtree(path, ignore_errors=True)

total = sum(kind != "known" for kind, *_ in cases)
print(f"{total - failed}/{total} passed, plus {len(cases) - total} known")
sys.exit(1 if failed else 0)
