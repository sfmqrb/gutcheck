# Speed and limits

## Speed (measured)

x86_64 laptop CPU (12th-gen Core i7, 20 threads), fp32, one line per forward pass, short lines:

| | |
|---|---|
| startup (load 1.3 GB model) | about 4 s |
| steady state | about 95 ms per line, roughly 10 lines/s |
| 1,050 lines, end to end | 102 s |

That is "interactive", not "milliseconds": the millisecond figures Laya publishes are for GPU or Apple Silicon. Batching did not help on CPU (compute-bound), and neither did picking a thread count (all cores is fastest). An int8 export is about 25% faster but changed the decision on 5 to 8 of 42 test lines, so it is not the default. Point gutcheck at a prefilter (`grep`, `head`) for big inputs; it is built for the hundreds-to-thousands of lines you would otherwise read by eye.

Identical lines are answered once and remembered (up to 50,000 distinct lines), so repetitive logs are much faster than the table above.

## Honest limits

- Zero-shot accuracy is modest. Laya's authors recommend fine-tuning for real workloads. Check it on your data before trusting it, and use `-s` to see the scores rather than trusting the 0.5 cut-off (a borderline bug report can score 0.49): on the demo files it misses some obvious cases (for example it rates "disable TLS certificate verification" as low risk).
- Phrasing matters. Try a second wording if a question misbehaves. `-c` with `label=description` helps.
- Lines only: one line is one record. Lines beyond 1,024 tokens are truncated.
