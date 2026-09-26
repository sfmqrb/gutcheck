# Models

gutcheck runs [Laya](https://github.com/NandhaKishorM/laya) decision models through ONNX Runtime. Pick one with `-M`:

| `-M` | Model | Size | Best for |
|---|---|---|---|
| `multilingual` (default) | mmBERT-base, 322M parameters | 1.3 GB | 100+ languages, the fastest |
| `english` | ModernBERT-large, 421M | 1.7 GB | English |
| `typed` | ModernBERT-large fine-tuned on incident, support-ticket, invoice and agent-trace decisions | 1.7 GB | those kinds of text, and the strongest ranking here |

Each is downloaded on first use to `$XDG_CACHE_HOME/gutcheck`.

## Measured

Zero-shot, 100 examples per set from public data (SST-2 sentiment, SMS spam, AG News topics). Binary sets show accuracy at the default 0.5 cut-off, then with `--auto`, then AUC (how well the scores rank positives above negatives). Reproduce with `python3 bench/accuracy.py`.

| model | SST-2 sentiment (100) | SMS spam (100) | AG News topic (100) | lines/s |
|---|--:|--:|--:|--:|
| multilingual | 50% → 79% (AUC 0.87) | 79% → 84% (AUC 0.94) | 95% | 9 |
| english | 53% → 64% (AUC 0.64) | 96% → 96% (AUC 0.99) | 95% | 4 |
| typed | 54% → 78% (AUC 0.89) | 84% → 85% (AUC 1.00) | 94% | 4 |

`lines/s` is steady-state speed on a laptop CPU with model loading factored out.

Read it like this: a decision model's scores are ordered well even when its raw probabilities run low, so `--auto`, `--rank` and `-i` are the way to use it on open-ended questions. `typed` ranks best (AUC 0.89 and 1.00); `multilingual` is more than twice as fast and the only one for non-English text, and `english` is strongest on spam-style questions. With `--auto`, all three reach 64 to 85% on the binary sets; news topics (`-c`) come out at 94 to 95% on every model.

## Named questions

Each of the 12 built-in questions was checked on hand-written examples (5 that should match, 5 that should not, for the 10 questions with a fixed test set). Accuracy at the default 0.5 cut-off:

| model | secret | pii | security | error | todo | frustrated | spam | bug | urgent | toxic | average |
|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| multilingual | 60% | 80% | 90% | 80% | 50% | 50% | 90% | 70% | 60% | 70% | 70% |
| english | 100% | 80% | 100% | 90% | 80% | 100% | 100% | 100% | 90% | 100% | 94% |
| typed | 100% | 80% | 100% | 80% | 80% | 100% | 100% | 100% | 100% | 100% | 94% |

That is why `@name` questions use `typed` by default (`-M` overrides). It is a small test set, so treat it as a guide, and try your own data.
