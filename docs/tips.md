# Tips

**Ask about what the text says.** Phrase the question as a statement to check: "does this log line describe an error?", "is the user frustrated?", "removes a security check".

**Let `--auto` pick the cut-off.** A decision model orders answers well even when its raw probabilities run low, so a fixed 0.5 can be too strict for open questions. `--auto` splits the scores into a yes group and a no group. Or use `-i` and slide the threshold yourself; `-s` and `--rank` show the scores.

**Use a named question.** `gutcheck --questions` lists the tested wordings for secrets, PII, errors, security, spam and more. Your own go in `~/.config/gutcheck/questions`.

**Sharpen labels.** `-c bug=crashes,billing=charges,question=how-to` gives each label a description.

**Rewrite once if needed.** A second wording of the same question often settles a borderline result.

**Logs: use `--fuzzy`.** Lines that differ only in numbers, timestamps and ids get one answer, 11 to 15 times faster on real logs. Skip it for questions about a value ("above 90%?").

**Big inputs: `--estimate` first.** Distinct text runs at about 10 lines per second on a laptop CPU. `--estimate` shows the calls a run needs; `grep` in front trims a big file to the part worth asking about.

**Long text.** Records are cut at the model's limit (1,024 tokens); raise it with `--max-tokens` for long documents. `-C N` shows the model the lines around each line.
