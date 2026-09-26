# gutcheck

**`grep` for meaning.** Ask a plain-English question about every line, paragraph, JSON record or diff hunk. One binary, runs on your CPU. No API key, no bill, nothing leaves your machine.

![demo](demo/demo.gif)

```sh
cargo install --git https://github.com/sfmqrb/gutcheck
```

Or take a binary (Linux x86_64, macOS arm64) from [releases](https://github.com/sfmqrb/gutcheck/releases). The first run downloads a 1.3 GB model to `$XDG_CACHE_HOME/gutcheck`.

## Use

```sh
gutcheck -n "is this a bug report?" issues.txt                 # keep matching lines, grep-style
gutcheck -s "does this log line describe an error?" app.log    # P(yes) for every line, colored
gutcheck -c bug,billing,question "what is this message about?" support.txt   # your own labels, any language
git diff | gutcheck --diff "removes a permission or security check"          # judge each hunk
gutcheck --field message "is the user frustrated?" events.jsonl              # judge a JSON field
gutcheck -rl --glob '*.md' "mentions a rent increase" notes/                 # search a directory
tail -f app.log | gutcheck -v "is this routine noise?"                       # streams; -v inverts
```

It follows grep where it can: `-n -H -l -q -m -v -r`, several questions with `-e` (add `--all` for "every one"), and exit codes 0 (match), 1 (none), 2 (error). Everything is in the [command reference](docs/cli.md).

## Fast on logs

The model is compute-bound (about 10 distinct lines per second), so gutcheck's speed comes from asking less. Identical lines are answered once, and with `-f` lines that differ only in numbers, timestamps and ids share one answer. On real logs ([Loghub](https://github.com/logpai/loghub) samples):

| log | distinct shapes in 2,000 lines | exact, 500 lines | `--fuzzy`, 500 lines | same decision |
|---|--:|--:|--:|--:|
| Apache | 12 | 57 s | 4 s | 99.8% |
| HDFS | 45 | 101 s | 9 s | 96.8% |
| Linux | 199 | 93 s | 9 s | 99.4% |

`gutcheck --estimate "..." big.log` tells you the number of model calls before you spend them. Method, and everything we tried that did *not* help: [benchmarks](docs/benchmarks.md).

## Know before you use it

Zero-shot accuracy is modest. Check it on your data, and use `-s` to see scores instead of trusting the 0.5 cut-off. Distinct free text runs at about 10 lines per second, so prefilter big inputs with `grep`. [Speed and limits](docs/speed-and-limits.md).

## More

[Command reference](docs/cli.md) · [Benchmarks](docs/benchmarks.md) · [gutcheck vs jgrep](docs/comparison.md) · [How it works and credits](docs/how-it-works.md)

gutcheck is independent and not affiliated with the Laya or Jev authors. MIT licensed; the Apache-2.0 model is downloaded at runtime.
