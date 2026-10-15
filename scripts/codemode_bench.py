#!/usr/bin/env python3
"""Run fixed tasks under each MINIMA_CODEMODE variant and tabulate cost and correctness.

Experiment only; see docs/dev/codemode.md. Needs a binary built with `--features codemode` and a
provider key, as for scripts/test_<provider>.sh. Each run gets a fresh `git archive HEAD` copy of
this repo as its root, so edit tasks never touch the checkout.

    cargo build --features codemode
    scripts/codemode_bench.py --provider anthropic --model claude-haiku-4-5-20251001 --reps 3
"""

import argparse
import json
import os
import re
import statistics
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VARIANTS = ["none", "line", "js", "js-only", "js-bash"]


def rs_files(root, sub="src"):
    return sorted(p for p in (root / sub).rglob("*.rs"))


def rel(root, p):
    return str(p.relative_to(root))


def count_tests(root):
    out = {}
    for p in rs_files(root):
        n = len(re.findall(r"^\s*#\[(?:tokio::)?test\b", p.read_text(), re.M))
        if n:
            out[rel(root, p)] = n
    return out


def check_count_tests(root, answer, _):
    total = sum(count_tests(root).values())
    m = re.search(r"total\D{0,10}(\d+)", answer, re.I)
    return bool(m) and int(m.group(1)) == total, f"expected total {total}"


def check_largest(root, answer, _):
    sizes = sorted(
        ((len(p.read_text().splitlines()), rel(root, p)) for p in rs_files(root)), reverse=True
    )[:3]
    missing = [f"{path}: {n}" for n, path in sizes if path not in answer or str(n) not in answer]
    return not missing, f"missing {missing}" if missing else ""


def undocumented(root):
    out = set()
    for p in rs_files(root, "src/tools"):
        lines = p.read_text().splitlines()
        for i, line in enumerate(lines):
            m = re.match(r"\s*pub (?:async )?fn (\w+)", line)
            if not m:
                continue
            j = i - 1
            while j >= 0 and lines[j].strip().startswith("#["):
                j -= 1
            if j < 0 or not lines[j].strip().startswith("///"):
                out.add((rel(root, p), m.group(1)))
    return out


def check_doc_gaps(root, answer, _):
    want = undocumented(root)
    lines = answer.splitlines()
    missing = [f"{p}:{n}" for p, n in want
               if not any(Path(p).name in l and re.search(rf"\b{n}\b", l) for l in lines)]
    return not missing, f"missing {missing}" if missing else ""


def check_rename(root, _answer, before):
    old = new = 0
    for p in rs_files(root):
        text = p.read_text()
        old += text.count("TOOL_OUTPUT_CAP")
        new += text.count("TOOL_RESULT_CAP")
    ok = old == 0 and new == before["TOOL_OUTPUT_CAP"]
    return ok, f"old={old} new={new} expected new={before['TOOL_OUTPUT_CAP']}"


def check_tokio(_root, answer, _):
    return "1.53" in answer, "expected 1.53"


TASKS = {
    # Aggregation over every source file: the case codemode is for.
    "count-tests": (
        "How many #[test] and #[tokio::test] functions does each .rs file under src/ contain? "
        "Answer with one 'path: N' line per file that has any, then 'total: N'.",
        check_count_tests,
    ),
    "largest": (
        "Which 3 .rs files under src/ (recursively) have the most lines? "
        "Answer 'path: lines', one per line.",
        check_largest,
    ),
    "doc-gaps": (
        "List every `pub fn` or `pub async fn` in .rs files under src/tools/ that has no `///` "
        "doc comment directly above it (attributes may sit in between). "
        "Answer 'path: name', one per line.",
        check_doc_gaps,
    ),
    # Multi-file edit.
    "rename": (
        "Rename the identifier TOOL_OUTPUT_CAP to TOOL_RESULT_CAP in every .rs file under src/. "
        "Do not build or run tests. Answer 'done' when finished.",
        check_rename,
    ),
    # Control: one small read, where codemode should not help.
    "control": ("Which version of tokio does Cargo.toml require? Answer with the version.", check_tokio),
}


def fixture():
    d = Path(tempfile.mkdtemp(prefix="minima-bench-"))
    archive = subprocess.run(["git", "-C", str(ROOT), "archive", "HEAD"], check=True, capture_output=True)
    subprocess.run(["tar", "-x", "-C", str(d)], input=archive.stdout, check=True)
    return d.resolve()


def run_one(args, variant, task):
    prompt, check = TASKS[task]
    root = fixture()
    before = {"TOOL_OUTPUT_CAP": sum(p.read_text().count("TOOL_OUTPUT_CAP") for p in rs_files(root))}
    cmd = [args.bin, "-p", prompt, "--json", "--root", str(root), "--max-turns", str(args.max_turns)]
    cmd += ["--provider", args.provider] if args.provider else []
    cmd += ["--model", args.model] if args.model else []
    cmd += ["--sandbox"] if args.sandbox else []
    cmd += ["--context", str(args.context)] if args.context else []
    env = dict(os.environ, MINIMA_CODEMODE=variant)
    try:
        proc = subprocess.run(cmd, env=env, capture_output=True, text=True, timeout=args.timeout)
        out = proc.stdout
    except subprocess.TimeoutExpired as e:
        out = (e.stdout or b"").decode() if isinstance(e.stdout, bytes) else (e.stdout or "")
    records = [json.loads(line) for line in out.splitlines() if line.startswith("{")]
    result = next((r for r in records if r.get("type") == "result"), {})
    calls = [r["name"] for r in records if r.get("type") == "tool_call"]
    failed = sum(1 for r in records if r.get("type") == "tool_result" and not r.get("ok"))
    ok, why = check(root, result.get("text", ""), before) if result.get("outcome") == "complete" else (False, result.get("error") or "no result")
    subprocess.run(["rm", "-rf", str(root)])
    return {
        "variant": variant, "task": task, "ok": ok, "why": why, "answer": result.get("text", ""),
        "outcome": result.get("outcome"), "turns": result.get("turns"),
        "input_tokens": result.get("input_tokens"), "output_tokens": result.get("output_tokens"),
        "cost": result.get("cost"), "calls": dict(Counter(calls)), "failed_calls": failed,
    }


def summarise(rows):
    print(f"\n{'task':12} {'variant':8} {'pass':>5} {'in_tok':>8} {'out_tok':>7} {'turns':>5} {'cost':>8}  calls")
    for task in TASKS:
        for variant in VARIANTS:
            rs = [r for r in rows if r["task"] == task and r["variant"] == variant]
            if not rs:
                continue
            med = lambda k: statistics.median([r[k] or 0 for r in rs])
            calls = Counter()
            for r in rs:
                calls.update(r["calls"])
            per = ", ".join(f"{k}={v / len(rs):.1f}" for k, v in sorted(calls.items()))
            passed = sum(r["ok"] for r in rs)
            print(f"{task:12} {variant:8} {passed:>2}/{len(rs):<2} {med('input_tokens'):>8.0f} "
                  f"{med('output_tokens'):>7.0f} {med('turns'):>5.0f} {med('cost'):>8.4f}  {per}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--bin", default=str(ROOT / "target/debug/minima"))
    ap.add_argument("--provider")
    ap.add_argument("--model")
    ap.add_argument("--variants", default=",".join(VARIANTS))
    ap.add_argument("--tasks", default=",".join(TASKS))
    ap.add_argument("--reps", type=int, default=3)
    ap.add_argument("--context", type=int, help="context window, for a local server")
    ap.add_argument("--max-turns", type=int, default=30)
    ap.add_argument("--timeout", type=int, default=600)
    ap.add_argument("--sandbox", action="store_true", help="pass --sandbox; default is the unsandboxed case")
    ap.add_argument("--out", default="codemode-bench.jsonl", help="one JSON line per run, appended")
    ap.add_argument("--summarise", metavar="JSONL", help="only print the table for an earlier --out")
    args = ap.parse_args()

    if args.summarise:
        summarise([json.loads(l) for l in Path(args.summarise).read_text().splitlines() if l.strip()])
        return
    variants = args.variants.split(",")
    tasks = args.tasks.split(",")
    for v in variants:
        if v not in VARIANTS:
            sys.exit(f"unknown variant {v}")
    for t in tasks:
        if t not in TASKS:
            sys.exit(f"unknown task {t}")

    rows = []
    with open(args.out, "a") as log:
        for rep in range(args.reps):
            for task in tasks:
                for variant in variants:
                    row = run_one(args, variant, task) | {"rep": rep}
                    rows.append(row)
                    log.write(json.dumps(row) + "\n")
                    log.flush()
                    mark = "ok  " if row["ok"] else "FAIL"
                    print(f"{mark} {task:12} {variant:8} in={row['input_tokens']} turns={row['turns']} "
                          f"calls={row['calls']} {'' if row['ok'] else row['why'][:120]}", flush=True)
    summarise(rows)


if __name__ == "__main__":
    main()
