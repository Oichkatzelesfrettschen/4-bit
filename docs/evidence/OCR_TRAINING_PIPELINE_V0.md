# Custom ONNX CTC Training Pipeline for OCR (v0)

**Date**: 2026-01-29
**Status**: DESIGN DOCUMENT
**Target**: Future implementation (post-Phase 5)

---

## Objective

Train lightweight ONNX CTC model for Intel MCS-4 chip label OCR (>98% accuracy target).

---

## Model Architecture

```
Input: (1, H, W) grayscale image (variable W, fixed H=32)
    |
    v
Conv Block 1: 32 filters, 3x3, ReLU, MaxPool(2,2)
    |
    v
Conv Block 2: 64 filters, 3x3, ReLU, MaxPool(2,1)
    |
    v
Reshape: (seq_len, 64)
    |
    v
LSTM: 128 hidden units, bidirectional
    |
    v
FC: vocab_size (26 letters + 10 digits + blank = 37)
    |
    v
CTC Loss
```

**Parameters**: ~150K (lightweight, <5 MB)

---

## Dataset

**Training Data**:
- 200+ labeled crops from all 4 chips
- Ground truth from manual annotation
- Augmentation: rotation (+/-5 deg), scaling (0.9-1.1x), noise

**Splits**:
- Train: 70% (140 crops)
- Validation: 15% (30 crops)
- Test: 15% (30 crops)

---

## Training

**Hyperparameters**:
- Optimizer: Adam (lr=0.001)
- Batch size: 16
- Epochs: 100 (early stopping on validation)
- Loss: CTC Loss

**Framework**: PyTorch -> export to ONNX

---

## Evaluation

**Metrics**:
- Character Error Rate (CER): <2%
- Word Accuracy: >98%

---

## Integration

```python
import onnxruntime as ort

session = ort.InferenceSession("ocr_model_v0.onnx")
output = session.run(None, {"input": image_tensor})
decoded_text = ctc_decode(output[0])
```

---

**Status**: DESIGN ONLY - DEFERRED TO FUTURE WORK (REQUIRES DATASET COLLECTION)
