#!/usr/bin/env python3
"""Accuracy of each gutcheck model on public labelled sets. Stdlib only.

    cargo build --release && python3 bench/accuracy.py [n=100] [models="multilingual english typed"]

Zero-shot: the question is the only thing the model gets. Binary sets show accuracy at the default 0.5\ncut-off / AUC (how well the scores rank positives above negatives); AG News shows accuracy. Sets:
  SST-2 (movie-review sentiment), SMS Spam, AG News (4 topics).
"""
import json, os, subprocess, sys, time, urllib.parse, urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
BIN = os.environ.get("GUTCHECK", os.path.join(HERE, "..", "target", "release", "gutcheck"))
N = int(sys.argv[1]) if len(sys.argv) > 1 else 100
MODELS = (sys.argv[2] if len(sys.argv) > 2 else "multilingual english typed").split()


def rows(dataset, config, split, offset, length):
    q = urllib.parse.urlencode(dict(dataset=dataset, config=config, split=split, offset=offset, length=length))
    with urllib.request.urlopen(f"https://datasets-server.huggingface.co/rows?{q}", timeout=60) as r:
        return [x["row"] for x in json.load(r)["rows"]]


def clean(t):
    return " ".join(t.split())


def balanced(rs, key, label_of, n):
    """n/2 of each binary label, in a stable order."""
    pos = [r for r in rs if label_of(r) == 1][: n // 2]
    neg = [r for r in rs if label_of(r) == 0][: n // 2]
    return sorted(pos + neg, key=lambda r: hash(r[key]) % 9973)


def load():
    cache = os.path.join(HERE, "data", f"accuracy_{N}.json")
    if os.path.exists(cache):
        return json.load(open(cache))
    os.makedirs(os.path.dirname(cache), exist_ok=True)
    sst = balanced(sum((rows("stanfordnlp/sst2", "default", "validation", o, 100) for o in range(0, 500, 100)), []), "sentence", lambda r: r["label"], N)
    spam = balanced(sum((rows("ucirvine/sms_spam", "plain_text", "train", o, 100) for o in range(0, 1500, 100)), []), "sms", lambda r: r["label"], N)
    per = max(N // 4, 1)
    ag = sum((rows("fancyzhx/ag_news", "default", "test", o, per) for o in range(0, 7600, 7600 // 4)), [])
    data = {
        "SST-2 sentiment": dict(q="is this movie review positive?", texts=[clean(r["sentence"]) for r in sst], y=[r["label"] for r in sst]),
        "SMS spam": dict(q="is this text message spam?", texts=[clean(r["sms"]) for r in spam], y=[r["label"] for r in spam]),
        "AG News topic": dict(q="what is this news article about?", labels=["world", "sports", "business", "technology"], texts=[clean(r["text"]) for r in ag], y=[r["label"] for r in ag]),
    }
    json.dump(data, open(cache, "w"))
    return data


def auc(scores, y):
    """Chance that a random positive outscores a random negative (ties count half)."""
    pos = [s for s, l in zip(scores, y) if l == 1]
    neg = [s for s, l in zip(scores, y) if l == 0]
    return sum((p > n) + 0.5 * (p == n) for p in pos for n in neg) / (len(pos) * len(neg))


def otsu(scores):
    """The same automatic cut-off as `gutcheck --auto`: best split of the log-odds into two groups."""
    import math
    z = sorted(math.log(min(max(p, 1e-4), 1 - 1e-4) / (1 - min(max(p, 1e-4), 1 - 1e-4))) for p in scores)
    n, total = len(z), sum(z)
    if n < 4 or z[-1] - z[0] < 1.0:
        return 0.5
    below, best, cut = 0.0, -1.0, 0.0
    for k in range(1, n):
        below += z[k - 1]
        between = k * (n - k) * (below / k - (total - below) / (n - k)) ** 2
        if between > best:
            best, cut = between, (z[k - 1] + z[k]) / 2
    return 1 / (1 + math.exp(-cut))


def speed(model, d):
    """Lines per second, with model loading factored out."""
    def t(n):
        start = time.time()
        subprocess.run([BIN, "-M", model, "-s", d["q"]], input="\n".join(d["texts"][:n]) + "\n", capture_output=True, text=True, check=True)
        return time.time() - start
    return 40 / (t(50) - t(10))


def run(model, d):
    args = [BIN, "-M", model]
    args += ["-c", ",".join(d["labels"]), d["q"]] if "labels" in d else ["-s", d["q"]]
    out = subprocess.run(args, input="\n".join(d["texts"]) + "\n", capture_output=True, text=True, check=True, env={**os.environ, "NO_COLOR": "1"}).stdout.splitlines()
    if "labels" in d:
        pred = [d["labels"].index(l.split("\t")[0]) for l in out]
        return f"{sum(p == y for p, y in zip(pred, d['y'])) / len(d['y']):.0%}"
    scores = [float(l.split("\t")[0]) for l in out]
    acc = lambda cut: sum((s >= cut) == bool(y) for s, y in zip(scores, d["y"])) / len(d["y"])
    return f"{acc(0.5):.0%} → {acc(otsu(scores)):.0%} (AUC {auc(scores, d['y']):.2f})"


data = load()
print("| model | " + " | ".join(f"{k} ({len(v['y'])})" for k, v in data.items()) + " | lines/s |")
print("|---|" + "--:|" * (len(data) + 1))
for m in MODELS:
    print(f"| {m} | " + " | ".join(run(m, d) for d in data.values()) + f" | {speed(m, next(iter(data.values()))):.0f} |")
    sys.stdout.flush()
