# OCR models (optional)

The default OCR pipeline uses **Tesseract** and does not require any ML model files.

To experiment with GPU-backed OCR, the repo supports a lightweight ONNX CTC backend via `onnxruntime`:

- Backend selection: `scripts/ocr_backend_v0.py::resolve_backend()`
- Enable by setting `OCR_ONNX_MODEL=/absolute/path/to/model.onnx`
- Prefer CUDA/TensorRT when available by passing `--prefer-cuda` (where supported by a script)

## Expected model contract

The ONNX backend assumes a CTC-style recognizer:

- Input: single-channel image as float32 in `[0,1]`, shape `(1, 1, H, W)` or `(1, H, W, 1)`
- Output: logits over classes including a blank, shape `(1, T, C)` or `(T, C)` (a few variants are normalized)
- Alphabet: defaults to `A–Z0–9` with `blank_id=36`

You can optionally provide a sidecar JSON named `<model>.onnx.json`:

```json
{ "alphabet": "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789", "blank_id": 36 }
```

No model is committed here; add one locally under this directory (gitignored) or point at an external path.

