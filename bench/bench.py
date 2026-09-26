#!/usr/bin/env python3
"""Exact vs --fuzzy on public log samples (Loghub 2k sets). Stdlib only.

    cargo build --release && python3 bench/bench.py            # about 5 minutes
"""
import os, re, subprocess, sys, time, urllib.request

BIN = os.environ.get("GUTCHECK", os.path.join(os.path.dirname(__file__), "..", "target", "release", "gutcheck"))
QUESTION = "does this log line describe an error?"
SETS = ["Apache", "HDFS", "Linux"]
HEAD = 500  # lines used for the exact-vs-fuzzy comparison; exact is slow


def sample(name):
    path = os.path.join(os.path.dirname(__file__), "data", f"{name}_2k.log")
    if not os.path.exists(path):
        os.makedirs(os.path.dirname(path), exist_ok=True)
        url = f"https://raw.githubusercontent.com/logpai/loghub/master/{name}/{name}_2k.log"
        urllib.request.urlretrieve(url, path)
    return path


def run(args, lines):
    t = time.time()
    out = subprocess.run([BIN, "-s", *args, QUESTION], input="\n".join(lines) + "\n", capture_output=True, text=True, check=True).stdout
    return time.time() - t, [float(l.split("\t")[0]) for l in out.splitlines()]


def shapes(lines):
    def tpl(l):
        return "".join("<hex>" if len(r) >= 8 and re.fullmatch(r"[0-9a-fA-F]+", r) and re.search(r"\d", r) else re.sub(r"\d+", "#", r)
                       for r in re.findall(r"[A-Za-z0-9]+|[^A-Za-z0-9]", l))
    return len({tpl(l) for l in lines})


print(f"| log | lines | distinct shapes | exact, first {HEAD} | --fuzzy, first {HEAD} | speedup | same decision | --fuzzy, all lines |")
print("|---|--:|--:|--:|--:|--:|--:|--:|")
for name in SETS:
    lines = open(sample(name), errors="replace").read().splitlines()
    te, exact = run([], lines[:HEAD])
    tf, fuzzy = run(["--fuzzy"], lines[:HEAD])
    ta, _ = run(["--fuzzy"], lines)
    same = sum((a >= .5) == (b >= .5) for a, b in zip(exact, fuzzy))
    print(f"| {name} | {len(lines)} | {shapes(lines)} | {te:.0f} s | {tf:.0f} s | {te / tf:.0f}x | {same / len(exact):.1%} | {ta:.0f} s |")
    sys.stdout.flush()
