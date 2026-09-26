# gutcheck

**`grep` for meaning.** Ask a plain-English question about every line, paragraph, JSON record or diff hunk. Ranked, colored, and answered on your own CPU: one binary, no API key, no bill, no data leaving your machine.

![demo](demo/demo.gif)

```sh
cargo install --git https://github.com/sfmqrb/gutcheck
```

Or take a binary (Linux x86_64, macOS arm64) from [releases](https://github.com/sfmqrb/gutcheck/releases). The first run downloads a 1.3 GB model to `$XDG_CACHE_HOME/gutcheck`.

## Use

```sh
gutcheck -n "is this a bug report?" issues.txt                       # keep the matching lines
gutcheck --top 5 -f "does this log line describe an error?" app.log  # the 5 most likely, ranked, with heat bars
gutcheck -c bug,billing,question --tally "what is this about?" support.txt   # a labelled histogram, any language
git diff | gutcheck --diff "removes a permission or security check"  # judge each hunk
gutcheck --field message "is the user frustrated?" events.jsonl      # judge one JSON field
gutcheck -rl --glob '*.md' "mentions a rent increase" notes/         # search a directory
tail -f app.log | gutcheck -v "is this routine noise?"               # streams; -v inverts
```

grep habits carry over: `-n -H -l -q -m -v -r`, several questions with `-e` (`--all` for "every one"), exit codes 0 (match), 1 (none), 2 (error). Pipes get plain text; a terminal gets color. All flags: [command reference](docs/cli.md).

## Made to be read

- **Heat bars.** `-s` and `--rank` show each score as a bar, hot to cold, so the answer is visible at a glance.
- **A receipt on every run.** `72 records · 8 model calls (64 reused) · 4.4 s`: you see what the run cost and what it saved.
- **`--rank` and `--top`.** No `sort -rn` needed; the best matches come first.
- **`--tally`.** Classify a file and get the distribution, one colored bar per label.
- **`--estimate`.** Know the number of model calls before running anything.

## Built for logs

Identical lines are answered once. With `-f`, lines that differ only in numbers, timestamps and ids share one answer, so a log with a few dozen line shapes costs a few dozen model calls. On real logs ([Loghub](https://github.com/logpai/loghub) samples):

| log | distinct shapes in 2,000 lines | exact, 500 lines | `--fuzzy`, 500 lines | same decision |
|---|--:|--:|--:|--:|
| Apache | 12 | 57 s | 4 s | 99.8% |
| HDFS | 45 | 101 s | 9 s | 96.8% |
| Linux | 199 | 93 s | 9 s | 99.4% |

Method: [benchmarks](docs/benchmarks.md). Getting the best answers: [tips](docs/tips.md).

## More

[Command reference](docs/cli.md) · [Benchmarks](docs/benchmarks.md) · [Tips](docs/tips.md) · [How it works and credits](docs/how-it-works.md)

gutcheck is an independent project, not affiliated with the Laya or Jev authors. MIT licensed; the Apache-2.0 model is downloaded at runtime.
