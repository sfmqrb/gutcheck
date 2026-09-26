# gutcheck

**`grep` for meaning.** Filter, score and classify lines with a plain-English question. One binary, runs locally on CPU. No API key, no Python, no server.

![demo](demo/demo.gif)

```sh
cargo install --git https://github.com/sfmqrb/gutcheck
```

Or download a binary (Linux x86_64, macOS arm64) from [releases](https://github.com/sfmqrb/gutcheck/releases). The first run downloads a 1.3 GB model to `$XDG_CACHE_HOME/gutcheck`.

## Use

```sh
# keep the lines that match
cat issues.txt | gutcheck "is this a bug report?"

# P(yes) for every line (colored on a terminal)
gutcheck -s "does this log line describe an error?" < app.log

# your own labels, any language
gutcheck -c bug,billing,question,praise "what is this message about?" < support.txt

# the opposite, with a stricter cut-off
tail -f app.log | gutcheck -v -t 0.7 "is this routine noise?"
```

One line in, one line out, streamed, so `tail -f` and `| head` work. Like grep, filter mode exits 1 when nothing matched. `NO_COLOR=1` turns colors off; pipes are never colored.

## Know before you use it

About 10 lines per second on a laptop CPU, and zero-shot accuracy is modest: check it on your data, and use `-s` to see scores instead of trusting the 0.5 cut-off. Details in [docs/speed-and-limits.md](docs/speed-and-limits.md).

## More

[How it works and credits](docs/how-it-works.md) · [Speed and limits](docs/speed-and-limits.md)

gutcheck is independent and not affiliated with the Laya or Jev authors. MIT licensed; the Apache-2.0 model is downloaded at runtime.
