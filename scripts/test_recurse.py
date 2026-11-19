#!/usr/bin/env python3
"""Live test of `-r`: a real model delegates to two children, waits for both, and adds up.

    scripts/test_recurse.py
    MODEL=claude-sonnet-5-5 scripts/test_recurse.py
    SANDBOX=off BUDGET=200000 scripts/test_recurse.py -P openrouter -m openai/gpt-5-nano

The provider is PROVIDER (default anthropic) and the model MODEL (default claude-haiku-5-5, the
cheapest Claude, and the stricter test of whether the prompt is followed), unless the arguments
name them; naming another provider leaves the model to minima. Arguments go to the root minima
as given; children inherit its provider and model. Keys come from wherever minima reads them.
Spends tokens: the family is capped at BUDGET (default 300000). Runs under --sandbox unless
SANDBOX=off.

The checks are what the mock tests cannot show: that a model reads the family block in the
system prompt and uses it. They read the root's output and the family's files after the run.
The run directory is kept and printed, with each child's log.
"""

import json
import os
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BIN = os.environ.get("BIN", os.path.join(ROOT, "target", "debug", "minima"))
SANDBOX = os.environ.get("SANDBOX", "on") != "off"
BUDGET = int(os.environ.get("BUDGET", "300000"))
TIMEOUT = int(os.environ.get("TIMEOUT", "600"))
PROVIDER = os.environ.get("PROVIDER", "anthropic")
MODEL = os.environ.get("MODEL", "claude-haiku-5-5")

LINES = {"a.txt": 17, "b.txt": 25}
PROMPT = (
    "Use child agents for this. Spawn one child to count the lines in a.txt and another to "
    "count the lines in b.txt; do not count them yourself. Collect both answers, then reply "
    "with only their sum, as a number."
)

results = []


def check(ok, what, detail=""):
    results.append(ok)
    print(f"{'PASS' if ok else 'FAIL'}  {what}" + (f": {detail}" if detail and not ok else ""))


def alive(pid):
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


def ledger(path):
    """Settled tokens and cost, and the count of holds nothing settled."""
    lines = []
    if os.path.exists(path):
        with open(path) as f:
            lines = [json.loads(l) for l in f if l.strip()]
    settled = {l["settles"] for l in lines if l.get("settles") is not None}
    open_holds = [i for i, l in enumerate(lines) if l.get("hold") and i not in settled]
    counted = [l for i, l in enumerate(lines) if not (l.get("hold") and i in settled)]
    return sum(l["tokens"] for l in counted), sum(l["cost"] for l in counted), len(open_holds)


def names(args, short, long):
    """Whether `args` gives the flag, in any of clap's spellings."""
    return any(a in (short, long) or a.startswith(long + "=") or (a.startswith(short) and len(a) > 2)
               for a in args)


def defaults(args):
    """The default provider and model, for each the arguments leave out. Clap refuses a flag
    given twice, so a default is added only when the arguments do not name it. MODEL belongs to
    PROVIDER: another provider without -m gets the model minima remembers for it."""
    if names(args, "-P", "--provider"):
        return []
    return ["-P", PROVIDER] + ([] if names(args, "-m", "--model") else ["-m", MODEL])


def main():
    if not os.access(BIN, os.X_OK):
        sys.exit(f"error: no binary at {BIN}; run 'make build' first")
    run = tempfile.mkdtemp(prefix="minima-recurse-live-")
    root = os.path.join(run, "root")
    state = os.path.join(run, "state")
    os.makedirs(root)
    for name, n in LINES.items():
        with open(os.path.join(root, name), "w") as f:
            f.writelines(f"line {i}\n" for i in range(1, n + 1))

    # --json: headless output cuts each command short, and the checks read whole commands.
    cmd = [BIN, "-r", "--budget-tokens", str(BUDGET), "--no-session", "--json"]
    cmd += ["--sandbox"] if SANDBOX else []
    args = sys.argv[1:]
    if "--mock" not in args:
        args = defaults(args) + args
    cmd += args + ["-p", PROMPT]
    # Only the state directory is redirected, so the family's files land in the run directory
    # and the config directory, with any keys, is the user's own.
    env = dict(os.environ, XDG_STATE_HOME=state)
    print(f"run directory: {run}")
    print(f"arguments: {' '.join(args)}")
    print(f"sandbox: {'on' if SANDBOX else 'off'}, budget: {BUDGET} tokens\n")
    try:
        out = subprocess.run(
            cmd, cwd=root, env=env, capture_output=True, text=True, timeout=TIMEOUT
        )
    except subprocess.TimeoutExpired:
        sys.exit(f"error: the root ran past {TIMEOUT}s; see {run}")
    with open(os.path.join(run, "root.log"), "w") as f:
        f.write(out.stdout + "\n--- stderr ---\n" + out.stderr)
    events = []
    for line in out.stdout.splitlines():
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            pass
    commands = []
    print("--- root transcript ---")
    for e in events:
        if e.get("type") == "tool_call" and e.get("name") == "bash":
            commands.append(json.loads(e["arguments"]).get("command", ""))
            print(f"$ {commands[-1]}")
        elif e.get("type") == "tool_result":
            body = (e.get("output") or "").rstrip()
            print("  " + body[:400].replace("\n", "\n  ") + (" ..." if len(body) > 400 else ""))
        elif e.get("type") == "turn" and e.get("text", "").strip():
            print(f"> {e['text'].strip()}")
    result = next((e for e in events if e.get("type") == "result"), {})
    if result.get("error"):
        print(f"error: {result['error']}")
    if out.stderr.strip():
        print("--- root stderr ---\n" + out.stderr.rstrip())
    print()

    families = os.path.join(state, "minima", "family")
    dirs = [os.path.join(families, d) for d in os.listdir(families)] if os.path.isdir(families) else []
    check(out.returncode == 0, "the root exits 0", f"exit {out.returncode}")
    check(len(dirs) == 1, "one family was founded", f"{len(dirs)}")
    if len(dirs) != 1:
        return finish(run)
    family = dirs[0]
    with open(os.path.join(family, "registry.json")) as f:
        members = json.load(f)["members"]
    children = [m for m in members if m["parent"] is not None]

    answer = (result.get("text") or "").strip()
    expected = str(sum(LINES.values()))
    check(expected in answer, f"the answer is {expected}", f"got {answer!r}")

    spawns = sum(c.count("-r spawn") for c in commands)
    waits = sum(c.count("-r wait") for c in commands)
    shutdowns = sum(c.count("-r shutdown") for c in commands)
    check(len(children) == 2, "two children were spawned", f"{len(children)} in the registry")
    check(waits >= len(children), "the root waited at least once per child", f"{waits} waits")
    check(shutdowns == 0, "the root never ran shutdown", f"{shutdowns} times")
    check(spawns == len(children), "every spawn command is in the registry", f"{spawns} commands")

    for m in children:
        answer_path = os.path.join(family, f"{m['id']}.answer")
        got = open(answer_path).read().strip() if os.path.exists(answer_path) else None
        check(got is not None, f"child {m['name']} wrote an answer")
        print(f"      {m['name']} ({m['model']}): {got!r}")
    check(all(m["ended_at"] is not None for m in members), "every member is marked ended")
    check(not any(alive(m["pid"]) for m in children), "no child process is left running")

    tokens, cost, holds = ledger(os.path.join(family, "ledger.jsonl"))
    check(holds == 0, "every hold was settled", f"{holds} open")
    check(tokens <= BUDGET, "spend is within the budget", f"{tokens} tokens")
    print(f"\nspent: {tokens} tokens, ${cost:.4f}, root turns: {result.get('turns')}")
    print(f"logs: {family}/<id>.log, root output: {run}/root.log")
    return finish(run)


def finish(run):
    failed = results.count(False)
    print(f"\n{len(results) - failed} passed, {failed} failed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
