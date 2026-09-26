# Benchmarks

Real logs from [Loghub](https://github.com/logpai/loghub) (public 2,000-line samples), the question *"does this log line describe an error?"*, on an x86_64 laptop CPU (12th-gen Core i7, 20 threads). Reproduce with `python3 bench/bench.py` (about 5 minutes; it downloads the samples).

| log | lines | distinct shapes | exact, first 500 | `--fuzzy`, first 500 | speedup | same decision | `--fuzzy`, all lines |
|---|--:|--:|--:|--:|--:|--:|--:|
| Apache | 2000 | 12 | 57 s | 4 s | 15x | 99.8% | 4 s |
| HDFS | 2000 | 45 | 101 s | 9 s | 11x | 96.8% | 18 s |
| Linux | 2000 | 199 | 93 s | 9 s | 11x | 99.4% | 35 s |

Times include about 4 s to load the model. "Same decision" is how often `--fuzzy` and the exact run agree on match / no match at the default threshold.

## Why it is fast, and where it stops

A model call costs about 0.1 s, so gutcheck's speed is about making fewer calls, and it makes very few.

- **Exact cache** (always on): a repeated line is answered once.
- **`--fuzzy`**: lines that differ only in digits, timestamps or long hex ids are the same shape and are answered once. Real logs are mostly a few dozen shapes, so 2,000 lines can cost 12 to 200 calls. Use it for questions about what a line *says*; for questions about a *value* ("is disk usage above 90%?") leave it off, since "85%" and "99%" share a shape. Free-text logs such as `journalctl` have more shapes and gain less.
- **`--estimate`** shows the number of calls before you run anything, and every run ends with a receipt: records, model calls, calls saved, seconds.
- **Early exit**: `-q`, `-l` and `-m` stop reading as soon as they have their answer.
- Empty lines are skipped without a call.
