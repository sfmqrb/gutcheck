# Benchmarks

Real logs from [Loghub](https://github.com/logpai/loghub) (public 2,000-line samples), the question *"does this log line describe an error?"*, on an x86_64 laptop CPU (12th-gen Core i7, 20 threads). Reproduce with `python3 bench/bench.py` (about 5 minutes; it downloads the samples).

| log | lines | distinct shapes | exact, first 500 | `--fuzzy`, first 500 | speedup | same decision | `--fuzzy`, all lines |
|---|--:|--:|--:|--:|--:|--:|--:|
| Apache | 2000 | 12 | 57 s | 4 s | 15x | 99.8% | 4 s |
| HDFS | 2000 | 45 | 101 s | 9 s | 11x | 96.8% | 18 s |
| Linux | 2000 | 199 | 93 s | 9 s | 11x | 99.4% | 35 s |

Times include about 4 s to load the model. "Same decision" is how often `--fuzzy` and the exact run agree on match / no match at the default threshold.

## Why it is fast, and where it stops

The model is compute-bound: one call costs about 0.1 s however short the line is, so the only way to go faster is to make fewer calls.

- **Exact cache** (always on): a repeated line is answered once.
- **`--fuzzy`**: lines that differ only in digits, timestamps or long hex ids are the same shape and are answered once. Real logs are mostly a few dozen shapes, so 2,000 lines can cost 12 to 200 calls. The price is that "85%" and "99%" look alike, so a question about *values* ("is disk usage above 90%?") should not use `--fuzzy`. Free-text logs (the Linux set, or `journalctl`) have many more shapes and gain less.
- **`--estimate`** tells you the number of calls before you spend them.
- **Early exit**: `-q`, `-l` and `-m` stop reading as soon as they have their answer.
- Empty lines are skipped without a call.

## What did not help

Measured and rejected, so you do not have to try them:

| Tried | Result |
|---|---|
| Fewer or more CPU threads (`--threads 8 / 12 / 16`) | no faster than all 20 |
| Capping tokens per record (`--max-tokens 128 / 256`) | no faster; 128 changed 2 of 150 decisions |
| Shorter option text in the prompt (`yes`/`no`) | 5% faster, changed 33 of 150 decisions |
| Batching lines into one call | no faster (compute-bound) |
| int8 model | 25% faster, changed 5 to 8 of 42 decisions |
| 4 processes x 5 threads | about 1.4x on distinct lines, at 4x the memory (1.3 GB each) |

Distinct free text stays at about 10 lines per second. For big inputs, prefilter with `grep`, then ask gutcheck.
