# Speed and limits

Speed numbers and what makes it faster: [benchmarks](benchmarks.md). Short version: about 10 lines per second on distinct text, 11 to 15 times faster on repetitive logs with `--fuzzy`.

## Honest limits

- Zero-shot accuracy is modest. Laya's authors recommend fine-tuning for real workloads. Check it on your data before trusting it, and use `-s` to see the scores rather than trusting the 0.5 cut-off (a borderline bug report can score 0.49): on the demo files it misses some obvious cases (for example it rates "disable TLS certificate verification" as low risk).
- Phrasing matters. Try a second wording if a question misbehaves. `-c` with `label=description` helps.
- Records beyond 1,024 tokens are truncated (`--max-tokens`). There is no `-C` context yet: each record is judged on its own.
