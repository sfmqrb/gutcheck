# gutcheck and jgrep

[jgrep](https://github.com/keltokhy/jgrep) is the closest relative: also "grep, but the pattern is a description". The difference is where the model runs.

| | gutcheck | jgrep |
|---|---|---|
| Model runs | on your CPU | TypeSafe's hosted Jev, OpenRouter, a gateway, or a local server you start |
| Needs | nothing (one binary, 1.3 GB model cached) | an API key and network, or a separate local server |
| Cost | free | pay per call (its README: about $0.012 for 994 lines) |
| Your text leaves the machine | never | yes, unless you run a local server |
| Speed on distinct lines | about 10 lines/s | about 200 lines/s at 32 requests in flight (its README: 994 titles in 4.6 s) |
| Speed on repetitive logs | 11 to 15x faster with `--fuzzy` (see benchmarks) | no template collapsing |
| Language | Rust | Python |

jgrep's numbers are from its README; we did not run it (it needs a key). Ours are in [benchmarks](benchmarks.md).

**Where gutcheck wins:** private or offline text, no bill, no key, and logs, where `--fuzzy` turns thousands of near-identical lines into a few dozen model calls.

**Where jgrep wins:** raw throughput on distinct text, the stronger hosted model, and features gutcheck does not have: `-C` context lines, CSV fields, function-level judging of Go/C/Python code, spending budgets and run records.

## What gutcheck covers

Files and `-r` with globs, `-n -H -l -q -m --count`, `-p`, several questions with `--all`, `--para`, `--whole`, JSON lines with `--field`, `--json`, `--diff`, `--estimate`, and grep's exit codes. One difference: in gutcheck `-c` picks a label from a list; the count is `--count`.
