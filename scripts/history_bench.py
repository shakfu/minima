#!/usr/bin/env python3
"""Inject a past minima bug into a copy of HEAD, let minima fix it, grade with HEAD's tests.

Each task is a bug from the CHANGELOG (or a synthetic one), reintroduced by exact string
replacements, plus an issue text that describes the symptom. Grading splices HEAD's test modules
and `tests/` back over the agent's work, then runs `cargo test`; a task is resolved when the whole
suite passes. History commits bundle several fixes each, so reverting a commit does not isolate one
bug; injecting into HEAD does.

Unlike SWE-bench the failing tests are visible: Rust unit tests share a file with the code. The
fixture drops CHANGELOG.md, which describes every fix.

    cargo build
    scripts/history_bench.py --check                      # validate tasks; no model calls
    scripts/history_bench.py --provider anthropic --model <model> --reps 3 --label base
    scripts/history_bench.py --summarise history-bench.jsonl
"""

import argparse
import difflib
import json
import os
import re
import shutil
import statistics
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
# Shared across fixtures, so only the minima crate rebuilds per run.
TARGET = ROOT / "target/history-bench"
TEST_MARK = "#[cfg(test)]\nmod tests {"

# name: (issue, [(path, old, new)], [tests the injection must break])
TASKS = {
    "edit-empty-old": (
        "`edit` with an empty `old` and `replace_all` rewrites the whole file instead of failing.",
        [("src/tools/edit.rs",
          "    // An empty pattern matches between every character, so replace_all would rewrite the file.\n"
          "    if args.old.is_empty() {\n"
          "        bail!(\"old is empty; name the exact text to replace\");\n"
          "    }\n",
          "")],
        ["tools::edit::tests::an_empty_old_is_refused_even_with_replace_all"],
    ),
    "json-text-on-failure": (
        "With `--json`, the `result` record's `text` holds the last turn's text even when the run "
        "failed or was cancelled after a turn that called tools. That text is the model's plan, "
        "not an answer.",
        [("src/frontend/json.rs",
          "    /// Text of the last completed round-trip. It is the answer only when the run completes: any\n"
          "    /// earlier round-trip in a run asked for tools.\n",
          "    /// Text of the last completed round-trip.\n"),
         ("src/frontend/json.rs", "        let complete = result.is_ok() && !cancelled;\n", ""),
         ("src/frontend/json.rs",
          '"text": if complete { self.last.as_str() } else { "" },', '"text": self.last,')],
        ["frontend::json::tests::a_failure_after_a_tool_turn_reports_no_answer"],
    ),
    "chat-calls-without-index": (
        "On openai-chat, a server that streams tool calls without an `index` gets them merged into "
        "one call with concatenated arguments.",
        [("src/provider/chat.rs",
          "                // Some servers omit the index. The id then tells calls apart, and a fragment with\n"
          "                // neither gets an empty key, which continues the call opened last.\n",
          ""),
         ("src/provider/chat.rs", "                    .or_else(|| call.id.clone())\n", "")],
        ["provider::chat::tests::calls_without_an_index_are_keyed_by_id"],
    ),
    "nameless-tool-call": (
        "A tool call streamed with an id or arguments but no name is dropped without notice. If it "
        "was the only call, the plan text is reported as the answer.",
        [("src/turn.rs",
          "                if p.id.is_empty() && p.arguments.is_empty() {\n"
          "                    continue;\n"
          "                }\n"
          "                bail!(\"the provider sent a tool call with no name (id {:?})\", p.id);\n",
          "                continue;\n")],
        ["turn::tests::a_call_that_never_got_a_name_is_an_error"],
    ),
    # Synthetic: never shipped. Tests localisation from a symptom with no CHANGELOG trail.
    "retry-after-jan-feb": (
        "A `Retry-After` header in HTTP-date form gives a wrong delay when the date falls in "
        "January or February.",
        [("src/provider/http.rs",
          "    let y = if month <= 2 { year - 1 } else { year };\n", "    let y = year;\n")],
        ["provider::http::tests::retry_after_takes_seconds_or_a_date"],
    ),
}

PROMPT = "Fix this bug in the source. Do not change the tests.\n\n{issue}"


def split_tests(text):
    i = text.find(TEST_MARK)
    return (text, "") if i < 0 else (text[:i], text[i:])


def fixture(task):
    d = Path(tempfile.mkdtemp(prefix="minima-hist-")).resolve()
    archive = subprocess.run(["git", "-C", str(ROOT), "archive", "HEAD"], check=True, capture_output=True)
    # -m: commit-time mtimes look older than the shared target's last build, so cargo would reuse
    # binaries built from another fixture.
    subprocess.run(["tar", "-x", "-m", "-C", str(d)], input=archive.stdout, check=True)
    (d / "CHANGELOG.md").unlink()
    for path, old, new in TASKS[task][1] if task else []:
        p = d / path
        code, tests = split_tests(p.read_text())
        if code.count(old) != 1:
            sys.exit(f"{task}: injection text occurs {code.count(old)} times in {path}")
        p.write_text(code.replace(old, new) + tests)
    return d


def snapshot(root):
    return {str(p.relative_to(root)): p.read_text()
            for sub in ("src", "tests") for p in (root / sub).rglob("*") if p.is_file()}


def restore_tests(root, before):
    """Put HEAD's tests back over the agent's work. Returns whether the agent had changed them."""
    touched = False
    for rel, text in before.items():
        p = root / rel
        if rel.startswith("tests/"):
            touched |= not p.exists() or p.read_text() != text
            p.parent.mkdir(parents=True, exist_ok=True)
            p.write_text(text)
            continue
        tests = split_tests(text)[1]
        if not tests or not p.exists():
            continue
        code, now = split_tests(p.read_text())
        touched |= now != tests
        p.write_text(code + tests)
    return touched


def cargo_test(root, timeout):
    env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET))
    try:
        proc = subprocess.run(["cargo", "test", "--no-fail-fast"], cwd=root, env=env,
                              capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return False, ["<timeout>"]
    failed = re.findall(r"^test (\S+) \.\.\. FAILED", proc.stdout, re.M)
    if proc.returncode and not failed:
        failed = ["<build failed>"]
    return proc.returncode == 0, failed


def patch(before, root):
    out = []
    for rel, text in before.items():
        p = root / rel
        now = p.read_text() if p.exists() else ""
        if now != text:
            out += difflib.unified_diff(text.splitlines(True), now.splitlines(True), rel, rel)
    return "".join(out)


def check(args):
    """Each task must break its named tests and nothing should fail at HEAD."""
    bad = 0
    for task in [None, *args.tasks]:
        root = fixture(task)
        ok, failed = cargo_test(root, args.test_timeout)
        shutil.rmtree(root)
        if task is None:
            print(f"{'ok  ' if ok else 'FAIL'} HEAD {'' if ok else failed}", flush=True)
            bad += not ok
            continue
        missing = [t for t in TASKS[task][2] if t not in failed]
        good = not ok and not missing
        bad += not good
        print(f"{'ok  ' if good else 'FAIL'} {task:26} breaks {failed}"
              + (f", expected {missing}" if missing else ""), flush=True)
    sys.exit(1 if bad else 0)


def run_one(args, task, config):
    root = fixture(task)
    before = snapshot(root)
    cmd = [args.bin, "-p", PROMPT.format(issue=TASKS[task][0]), "--json", "--root", str(root),
           "--max-turns", str(args.max_turns)]
    cmd += ["--provider", args.provider] if args.provider else []
    cmd += ["--model", args.model] if args.model else []
    cmd += ["--sandbox", "--writable", str(TARGET)] if args.sandbox else []
    cmd += ["--context", str(args.context)] if args.context else []
    # The user's own AGENTS.md and skills would otherwise reach every prompt.
    env = dict(os.environ, CARGO_TARGET_DIR=str(TARGET), XDG_CONFIG_HOME=config)
    try:
        out = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=args.timeout).stdout
    except subprocess.TimeoutExpired as e:
        out = e.stdout.decode() if isinstance(e.stdout, bytes) else (e.stdout or "")
    records = [json.loads(line) for line in out.splitlines() if line.startswith("{")]
    result = next((r for r in records if r.get("type") == "result"), {})
    calls = [r["name"] for r in records if r.get("type") == "tool_call"]
    diff = patch(before, root)
    touched = restore_tests(root, before)
    ok, failed = cargo_test(root, args.test_timeout)
    shutil.rmtree(root)
    return {
        "label": args.label, "task": task, "ok": ok, "failed_tests": failed,
        "touched_tests": touched, "outcome": result.get("outcome"), "error": result.get("error"),
        "turns": result.get("turns"), "input_tokens": result.get("input_tokens"),
        "output_tokens": result.get("output_tokens"), "cost": result.get("cost"),
        "calls": dict(Counter(calls)),
        "failed_calls": sum(1 for r in records if r.get("type") == "tool_result" and not r.get("ok")),
        "patch": diff,
    }


def summarise(rows):
    print(f"\n{'label':10} {'task':26} {'pass':>5} {'in_tok':>8} {'out_tok':>7} {'turns':>5} "
          f"{'cost':>8} {'tamper':>6}")
    for label in dict.fromkeys(r["label"] for r in rows):
        mine = [r for r in rows if r["label"] == label]
        for task in dict.fromkeys(r["task"] for r in mine):
            rs = [r for r in mine if r["task"] == task]
            med = lambda k: statistics.median([r[k] or 0 for r in rs])
            print(f"{label:10} {task:26} {sum(r['ok'] for r in rs):>2}/{len(rs):<2} "
                  f"{med('input_tokens'):>8.0f} {med('output_tokens'):>7.0f} {med('turns'):>5.0f} "
                  f"{med('cost'):>8.4f} {sum(r['touched_tests'] for r in rs):>6}")
        cost = sum(r["cost"] or 0 for r in mine)
        print(f"{label:10} {'all':26} {sum(r['ok'] for r in mine):>2}/{len(mine):<2} total cost {cost:.4f}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--bin", default=str(ROOT / "target/debug/minima"))
    ap.add_argument("--provider")
    ap.add_argument("--model")
    ap.add_argument("--label", default="default", help="tags rows, to compare configurations")
    ap.add_argument("--tasks", default=",".join(TASKS))
    ap.add_argument("--reps", type=int, default=1)
    ap.add_argument("--context", type=int, help="context window, for a local server")
    ap.add_argument("--max-turns", type=int, default=50)
    ap.add_argument("--timeout", type=int, default=1200, help="seconds per agent run")
    ap.add_argument("--test-timeout", type=int, default=900, help="seconds per cargo test")
    ap.add_argument("--sandbox", action="store_true", help="pass --sandbox; default is unsandboxed")
    ap.add_argument("--check", action="store_true", help="validate the tasks without a model")
    ap.add_argument("--out", default="history-bench.jsonl", help="one JSON line per run, appended")
    ap.add_argument("--summarise", metavar="JSONL", help="only print the table for an earlier --out")
    args = ap.parse_args()

    if args.summarise:
        summarise([json.loads(l) for l in Path(args.summarise).read_text().splitlines() if l.strip()])
        return
    args.tasks = args.tasks.split(",")
    for t in args.tasks:
        if t not in TASKS:
            sys.exit(f"unknown task {t}")
    if args.check:
        check(args)

    rows = []
    config = tempfile.mkdtemp(prefix="minima-hist-config-")
    try:
        with open(args.out, "a") as log:
            for rep in range(args.reps):
                for task in args.tasks:
                    row = run_one(args, task, config) | {"rep": rep}
                    rows.append(row)
                    log.write(json.dumps(row) + "\n")
                    log.flush()
                    mark = "ok  " if row["ok"] else "FAIL"
                    print(f"{mark} {task:26} turns={row['turns']} calls={row['calls']} "
                          f"{'' if row['ok'] else row['failed_tests'][:5]}", flush=True)
    finally:
        shutil.rmtree(config)
    summarise(rows)


if __name__ == "__main__":
    main()
