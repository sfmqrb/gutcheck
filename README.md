# gutcheck

**`grep` for meaning.** Ask a plain-English question about every line, paragraph, JSON record or diff hunk. Runs on your CPU: no API key, nothing leaves your machine.

![demo](demo/demo.gif)

```sh
curl -fsSL https://raw.githubusercontent.com/sfmqrb/gutcheck/main/install.sh | sh   # Linux x86_64, macOS arm64
brew install sfmqrb/tap/gutcheck
cargo install --git https://github.com/sfmqrb/gutcheck
```

The first run downloads a 1.3 GB model.

```sh
gutcheck -n "is this a bug report?" issues.txt                  # filter, grep-style
gutcheck -i "does this log line describe an error?" app.log     # explore the scores live
gutcheck @secret -r src/                                        # named questions
gutcheck --top 5 -f "is this an attack?" access.log             # rank; -f is 10x faster on logs
gutcheck -c bug,billing,question --tally "what is this about?" support.txt
git diff | gutcheck --diff "removes a security check"
```

grep flags carry over: `-n -H -l -q -m -v -r -C`, exit codes 0, 1, 2.

[Command reference](docs/cli.md) · [Models](docs/models.md) · [Benchmarks](docs/benchmarks.md) · [Tips](docs/tips.md) · [How it works](docs/how-it-works.md)

MIT. Not affiliated with the Laya or Jev authors; the Apache-2.0 models are downloaded at runtime.
