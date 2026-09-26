# Command reference

```
gutcheck [OPTIONS] <QUESTION> [PATH]...
gutcheck [OPTIONS] -e <QUESTION>... [PATH]...
```

The first argument is the question; the rest are files (stdin if there are none, or `-`). With `-e`, every argument is a path. A question can be a name: `@secret`, `@error`... (see [Named questions](#named-questions)).

## What to judge

| Option | |
|---|---|
| *(default)* | Each line. |
| `-C N` | Each line, with `N` lines either side shown to the model. Reads the whole input first. |
| `--para` | Each paragraph (blank-line separated). |
| `--whole` | Each whole file. |
| `--field NAME` | One field of each JSON line (`a.b` for nested). Prints the whole line. |
| `--field NAME --csv` | One column of a CSV file with a header row. Prints the whole row. |
| `--diff` | Each hunk of a unified diff (`git diff`, `git log -p`). Prints the hunk with its file name. |
| `-r`, `--glob GLOB`, `--no-ignore` | Search directories. Respects `.gitignore`; `--glob '*.rs'` includes, `--glob '!*.md'` excludes. Binary files are skipped. |

## What to ask

| Option | |
|---|---|
| `-e Q` | Another question. A line matches if any question fits, or all with `--all`. |
| `-p P` (`-t`) | Match when P(yes) is at least `P`. Default 0.5. |
| `--auto` | Find the cut-off from the scores themselves (splits them into a "yes" group and a "no" group). Reads everything first. |
| `-v` | Keep the lines that do not match. |
| `-s` | Print every record with its P(yes), with a heat bar on a terminal. |
| `--rank`, `--top N` | Print records ranked by P(yes), best first; `--top` keeps the best `N`. |
| `-c A,B,C` | Print every record with the best label. `label=description` sharpens a label. |
| `-M MODEL` | `multilingual` (default), `english` or `typed`. Named questions use `typed` unless you say otherwise. See [models](models.md). |

## What to print

| Option | |
|---|---|
| `-n`, `-H` | Line number, file name (default with several files), as in grep. |
| `--count` | Only the number of matches per file. |
| `-l` | Only names of files with a match, stopping each file at its first match. |
| `-q` | Print nothing, stop at the first match. |
| `-m N` | Stop each file after `N` matches. |
| `--tally` | With `-c`: a histogram of the labels instead of every record. |
| `--json` | One JSON object per printed record: `file`, `line`, `text`, plus `p` or `label`. |

On a terminal, a run ends with a receipt on stderr: `72 records · 8 model calls (64 reused) · 4.4 s`. Pipes get plain text, no colors and no receipt.

## Interactive

`-i` opens a live view: every record with its score, best first. `←`/`→` move the threshold, `/` asks a new question, `s` toggles sorting, `f` toggles the filter, `↑`/`↓` scroll, `enter` prints the matches and exits, `q` quits. Works on files and pipes: `git diff | gutcheck -i --diff "..."`.

## Named questions

`@error`, `@security`, `@secret`, `@pii`, `@bug`, `@feature`, `@spam`, `@frustrated`, `@urgent`, `@toxic`, `@todo`, `@question`. They run on the `typed` model, which scores 94% on them against 70% for `multilingual` ([measured](models.md#named-questions)). List them with `gutcheck --questions`. Add your own in `~/.config/gutcheck/questions`, one `name = question` per line:

```
slow = does this describe something slow?
```

## Speed

| Option | |
|---|---|
| `-f`, `--fuzzy` | Lines that differ only in numbers, timestamps and ids are answered once. See [benchmarks](benchmarks.md). |
| `--estimate` | Read the input and report how many model calls it needs, with and without `--fuzzy`. Does not load the model. |
| `--gpu` | Run on the Apple GPU and Neural Engine through Core ML (macOS builds). |
| `--max-tokens N`, `--threads N` | Cap tokens per record; CPU threads. |

Identical lines are always answered once, and empty records are skipped without a model call.

## Exit status

Like grep: `0` if something matched, `1` if nothing did, `2` on error. `-s`, `-c` and `--rank` always print, so they exit `0`.

## Shell integration

`gutcheck --completions bash|zsh|fish|elvish|powershell` prints a completion script; `gutcheck --man` prints the man page.

## Environment

`NO_COLOR=1` turns colors off (they only appear on a terminal), `CLICOLOR_FORCE=1` forces them. `GUTCHECK_MODEL=/path/model.onnx` uses another ONNX export. Models are cached in `$XDG_CACHE_HOME/gutcheck`.

## Examples

```sh
gutcheck -n "is this a bug report?" issues.txt notes.txt
gutcheck @secret -r src/                                           # hunt for credentials
gutcheck -rl --glob '*.md' "mentions a rent increase" notes/
gutcheck -e "about economics" -e "about New York" --all article.txt
gutcheck --field message --json "does this describe a failed payment?" events.jsonl
gutcheck --csv --field review "is the customer angry?" reviews.csv
git diff | gutcheck --diff "removes a permission or security check"
gutcheck -q "is this a stack trace?" build.log && notify-send "build broke"
```
