# Emulator image assets

This directory contains the original bitmap assets (`*.bmp`) used by the i400x emulator tooling, plus **PNG copies** generated for easier viewing and annotation.

Notes:

- Some tools (including Codex UI) cannot render/attach `*.bmp`/`*.pbm`; use the corresponding `*.png` instead.
- The bitmap files are kept as the “source-of-truth” originals; PNGs are derived artifacts.
- To (re)generate PNGs from any `bmp/pbm/pgm/ppm` files:
  - `python scripts/convert_emulator_images_v0.py --in-dir docs/emulators --force`

