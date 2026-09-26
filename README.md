# gutcheck

**`grep` for meaning.** Filter, score and classify lines with a plain-English question, from a single binary, locally on CPU. No API key, no Python, no server.

![demo](demo/demo.gif)

```sh
cargo install --git https://github.com/sfmqrb/gutcheck
```

Prebuilt binaries are attached to each GitHub release. The first run downloads a 1.3 GB model to `$XDG_CACHE_HOME/gutcheck`.

## Examples

```sh
# keep only what matches the question
cat issues.txt | gutcheck "is this a bug report?"

# score every line with P(yes) and rank them
git log --oneline | gutcheck -s "could this commit break prod or weaken security?" | sort -rn | head

# classify into your own labels (any language; `label=description` sharpens a label)
journalctl -o cat | gutcheck -c "error,warning,info" "what is the log level of this line?"

# keep what does NOT match, and tune the cut-off
tail -f app.log | gutcheck -v -t 0.7 "is this routine noise?"

# multilingual out of the box
gutcheck -c bug,billing,question,praise "what is this message about?" < examples/support.txt
```

Output is one line per input line, in order, streamed (so `tail -f` and `| head` work). `-s` prints `P(yes)<TAB>line`, `-c` prints `label<TAB>line`, and no flag filters (exit 0 always, unlike grep).

## How it works

gutcheck runs a **decision model**: a bidirectional encoder that reads `[question] [options] [text]` once and reads the answer off small scoring heads. There is no text generation, so there is nothing to parse and nothing to hallucinate, and the score is a probability. Yes/no questions use the model's `noul` head, labelled choices use its `choice` head.

The model is [Laya](https://github.com/NandhaKishorM/laya) multilingual (mmBERT-base, 322M parameters, 100+ languages), run through [ONNX Runtime](https://github.com/pykeio/ort) with the community ONNX export [soyelmismo/laya-multilingual-onnx](https://huggingface.co/soyelmismo/laya-multilingual-onnx), pinned to one revision. The prompt layout is a port of Laya's `build_sequence`; the answers were checked against the reference Python `laya` package on the demo files. Set `GUTCHECK_MODEL=/path/to/model.onnx` to try another export.

## Speed (measured)

x86_64 laptop CPU (12th-gen Core i7, 20 threads), fp32, one line per forward pass, short lines:

| | |
|---|---|
| startup (load 1.3 GB model) | about 4 s |
| steady state | about 95 ms per line, roughly 10 lines/s |
| 1,050 lines, end to end | 102 s |

That is "interactive", not "milliseconds": the millisecond figures Laya publishes are for GPU or Apple Silicon. Batching did not help on CPU (compute-bound), and neither did picking a thread count (all cores is fastest). An int8 export is about 25% faster but changed the decision on 5 to 8 of 42 test lines, so it is not the default. Point gutcheck at a prefilter (`grep`, `head`) for big inputs; it is built for the hundreds-to-thousands of lines you would otherwise read by eye.

## Honest limits

- Zero-shot accuracy is modest. Laya's authors recommend fine-tuning for real workloads. Check it on your data before trusting it: on the demo files it misses some obvious cases (for example it rates "disable TLS certificate verification" as low risk).
- Phrasing matters. Try a second wording if a question misbehaves. `-c` with `label=description` helps.
- Lines only: one line is one record. Lines beyond 1,024 tokens are truncated.

## Credits and licenses

gutcheck is an independent project, not affiliated with or endorsed by Convai Innovations, TypeSafe, or the authors of the projects below.

- [Laya](https://github.com/NandhaKishorM/laya) by Nandakishor M / Convai Innovations, the decision model (weights Apache-2.0). The weights are downloaded on first run, not redistributed here.
- [soyelmismo](https://huggingface.co/soyelmismo/laya-multilingual-onnx), who published the ONNX export used here (see also [mizorewww/laya-mlx](https://github.com/mizorewww/laya-mlx) for the Apple Silicon port).
- Jev, TypeSafe's hosted "System One" service, popularised typed decision models. See also [Kev](https://github.com/jaredpalmer/kev) and [SemIf](https://github.com/TheoLeeCJ/SemIf-OpenJev) for other open takes.

gutcheck's code is MIT (see `LICENSE`). Apache-2.0 model weights are fetched separately at runtime, so the licenses do not conflict.
