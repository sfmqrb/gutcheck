# Command reference

```
gutcheck [OPTIONS] <QUESTION> [PATH]...
gutcheck [OPTIONS] -e <QUESTION>... [PATH]...
```

The first argument is the question; the rest are files (stdin if there are none, or `-`). With `-e`, every argument is a path.

## What to judge

| Option | |
|---|---|
| *(default)* | Each line. |
| `--para` | Each paragraph (blank-line separated). |
| `--whole` | Each whole file. |
| `--field NAME` | One field of each JSON line (`a.b` for nested). Prints the whole line. |
| `--diff` | Each hunk of a unified diff (`git diff`, `git log -p`). Prints the hunk with its file name. |
| `-r`, `--glob GLOB`, `--no-ignore` | Search directories. Respects `.gitignore`; `--glob '*.rs'` includes, `--glob '!*.md'` excludes. Binary files are skipped. |

## What to ask

| Option | |
|---|---|
| `-e Q` | Another question. A line matches if any question fits, or all with `--all`. |
| `-p P` (`-t`) | Match when P(yes) is at least `P`. Default 0.5. |
| `-v` | Keep the lines that do not match. |
| `-s` | Print every record with its P(yes), colored by heat. |
| `-c A,B,C` | Print every record with the best label. `label=description` sharpens a label. |

## What to print

| Option | |
|---|---|
| `-n`, `-H` | Line number, file name (default with several files), as in grep. |
| `--count` | Only the number of matches per file. |
| `-l` | Only names of files with a match, stopping each file at its first match. |
| `-q` | Print nothing, stop at the first match. |
| `-m N` | Stop each file after `N` matches. |
| `--json` | One JSON object per printed record: `file`, `line`, `text`, plus `p` or `label`. |

## Speed

| Option | |
|---|---|
| `-f`, `--fuzzy` | Lines that differ only in numbers, timestamps and ids are answered once. See [benchmarks](benchmarks.md). |
| `--estimate` | Read the input and report how many model calls it needs, with and without `--fuzzy`. Does not load the model. |
| `--max-tokens N`, `--threads N` | Cap tokens per record; CPU threads. Neither changed speed in our tests. |

Identical lines are always answered once, and empty records are skipped without a model call.

## Exit status

Like grep: `0` if something matched, `1` if nothing did, `2` on error. `-s` and `-c` always print, so they exit `0`.

## Environment

`NO_COLOR=1` turns colors off (they only appear on a terminal), `CLICOLOR_FORCE=1` forces them. `GUTCHECK_MODEL=/path/model.onnx` uses another ONNX export. The model is cached in `$XDG_CACHE_HOME/gutcheck`.

## Examples

```sh
gutcheck -n "is this a bug report?" issues.txt notes.txt
gutcheck -rl --glob '*.md' "mentions a rent increase" notes/
gutcheck -e "about economics" -e "about New York" --all article.txt
gutcheck --field message --json "does this describe a failed payment?" events.jsonl
git diff | gutcheck --diff "removes a permission or security check"
gutcheck -q "is this a stack trace?" build.log && notify-send "build broke"
```
