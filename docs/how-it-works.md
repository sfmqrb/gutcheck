# How it works

gutcheck runs a **decision model**: a bidirectional encoder that reads `[question] [options] [text]` once and reads the answer off small scoring heads. There is no text generation, so there is nothing to parse and nothing to hallucinate, and the score is a probability. Yes/no questions use the model's `noul` head, labelled choices use its `choice` head.

The model is [Laya](https://github.com/NandhaKishorM/laya) multilingual (mmBERT-base, 322M parameters, 100+ languages), run through [ONNX Runtime](https://github.com/pykeio/ort) with the community ONNX export [soyelmismo/laya-multilingual-onnx](https://huggingface.co/soyelmismo/laya-multilingual-onnx), pinned to one revision. The prompt layout is a port of Laya's `build_sequence`; the answers were checked against the reference Python `laya` package on the demo files. Set `GUTCHECK_MODEL=/path/to/model.onnx` to try another export.

## Credits and licenses

gutcheck is an independent project, not affiliated with or endorsed by Convai Innovations, TypeSafe, or the authors of the projects below.

- [Laya](https://github.com/NandhaKishorM/laya) by Nandakishor M / Convai Innovations, the decision model (weights Apache-2.0). The weights are downloaded on first run, not redistributed here.
- [soyelmismo](https://huggingface.co/soyelmismo/laya-multilingual-onnx), who published the ONNX export used here (see also [mizorewww/laya-mlx](https://github.com/mizorewww/laya-mlx) for the Apple Silicon port).
- Jev, TypeSafe's hosted "System One" service, popularised typed decision models. See also [Kev](https://github.com/jaredpalmer/kev) and [SemIf](https://github.com/TheoLeeCJ/SemIf-OpenJev) for other open takes.

gutcheck's code is MIT (see `LICENSE`). Apache-2.0 model weights are fetched separately at runtime, so the licenses do not conflict.
