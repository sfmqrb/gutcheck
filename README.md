# gutcheck

**`grep` for meaning.** Ask a plain-English question about every line, paragraph, JSON record or diff hunk, then explore the answers in a live terminal view. One binary, runs on your CPU. No API key, no bill, nothing leaves your machine.

![demo](demo/demo.gif)

```sh
curl -fsSL https://raw.githubusercontent.com/sfmqrb/gutcheck/main/install.sh | sh   # Linux x86_64, macOS arm64
brew install sfmqrb/tap/gutcheck                                                    # Homebrew
cargo install --git https://github.com/sfmqrb/gutcheck                              # from source
```

The first run downloads a 1.3 GB model to `$XDG_CACHE_HOME/gutcheck`.

## Use

```sh
gutcheck -n "is this a bug report?" issues.txt             # keep the matching lines, grep-style
gutcheck -i "does this log line describe an error?" app.log   # explore live: threshold, sort, re-ask
gutcheck @secret -r src/                                   # named questions: hunt credentials in a repo
gutcheck --top 5 -f "..." app.log                          # the 5 most likely, ranked, with heat bars
gutcheck -c bug,billing,question --tally "what is this about?" support.txt   # a labelled histogram
git diff | gutcheck --diff "removes a permission or security check"          # judge each hunk
gutcheck --field message "is the user frustrated?" events.jsonl              # JSON field (or --csv)
tail -f app.log | gutcheck -v "is this routine noise?"                      # streams; -v inverts
```

grep habits carry over: `-n -H -l -q -m -v -r -C`, several questions with `-e` (`--all` for "every one"), exit codes 0 (match), 1 (none), 2 (error). Pipes get plain text; a terminal gets color. Every flag: [command reference](docs/cli.md).

## Made to be explored

- **`-i`, a live view.** Scores appear as they arrive, best first. Slide the threshold with the arrow keys, press `/` to ask a new question, `enter` to print the matches.
- **`--auto`.** Finds the cut-off from the scores themselves, so you don't guess a number.
- **Named questions.** `@secret`, `@pii`, `@security`, `@error`, `@bug`, `@spam`... or your own in `~/.config/gutcheck/questions`.
- **Heat bars, `--rank`, `--tally`.** Answers you can read at a glance.
- **A receipt on every run.** `72 records · 8 model calls (64 reused) · 4.4 s`: what the run cost and what it saved.
- **Three models.** `-M multilingual` (100+ languages, fastest), `english`, and `typed`, the most accurate on English (named questions use it). [Models and accuracy](docs/models.md).

## Built for logs

Identical lines are answered once. With `-f`, lines that differ only in numbers, timestamps and ids share one answer, so a log with a few dozen line shapes costs a few dozen model calls. On real logs ([Loghub](https://github.com/logpai/loghub) samples):

| log | distinct shapes in 2,000 lines | exact, 500 lines | `--fuzzy`, 500 lines | same decision |
|---|--:|--:|--:|--:|
| Apache | 12 | 57 s | 4 s | 99.8% |
| HDFS | 45 | 101 s | 9 s | 96.8% |
| Linux | 199 | 93 s | 9 s | 99.4% |

`gutcheck --estimate "..." big.log` shows the number of model calls before you spend them. Method: [benchmarks](docs/benchmarks.md). Getting the best answers: [tips](docs/tips.md).

## More

[Command reference](docs/cli.md) · [Models and accuracy](docs/models.md) · [Benchmarks](docs/benchmarks.md) · [Tips](docs/tips.md) · [How it works and credits](docs/how-it-works.md)

gutcheck is an independent project, not affiliated with the Laya or Jev authors. MIT licensed; the Apache-2.0 models are downloaded at runtime.
