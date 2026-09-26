# Tips

**Ask about what the text says.** Phrase the question as a statement to check: "does this log line describe an error?", "is the user frustrated?", "removes a security check".

**Tune the cut-off with `-s`.** `-s` shows every score, and `-p 0.7` (stricter) or `-p 0.3` (looser) moves the line. A borderline match can score 0.49; `--rank` shows you the whole order.

**Sharpen labels.** `-c bug=crashes,billing=charges,question=how-to` gives each label a description.

**Rewrite once if needed.** A second wording of the same question often settles a borderline result.

**Logs: use `--fuzzy`.** Lines that differ only in numbers, timestamps and ids get one answer, 11 to 15 times faster on real logs. Skip it for questions about a value ("above 90%?").

**Big inputs: `--estimate` first.** Distinct text runs at about 10 lines per second on a laptop CPU. `--estimate` shows the calls a run needs; `grep` in front trims a big file to the part worth asking about.

**Long records** are cut at 1,024 tokens (`--max-tokens`). Each record is judged on its own; there is no `-C` context yet.
