# Layer Annotations

Purpose
- Provide annotated overlays to help interpret MCS-4 mask layers and schematics.

## 4004 schematic bus labels
- Source: docs/emulators/i4004-schematic.bmp
- Signal map: docs/emulators/i4004-signals.txt
- Output: docs/4004/annotated/i4004-schematic-bus-labels.png

Method
- Parsed coordinates from i4004-signals.txt for CLK1/CLK2, SYNC, CMROM, CMRAM0-3, D0-D3, and D0-3 pads.
- Drew labeled markers at those coordinates on the schematic image.

Notes
- Labels are based on the signal map used by the analyzer, not hand-interpreted by eye.
- This overlay is schematic-level, not layout-level.

## 4004 transistor overlay (poly + diffusion)
- Sources: docs/emulators/i4004-poly.bmp, docs/emulators/i4004-diffusion.bmp
- Output: docs/4004/annotated/i4004-poly-diffusion-transistors.png

Method
- Thresholded poly and diffusion masks to binary layers.
- Colored poly red and diffusion green.
- Highlighted intersections (poly + diffusion) in yellow to indicate transistor sites.

Notes
- This is an algorithmic overlay; verify against original masks for precise analysis.
- Does not include metal/via connectivity, so it is not a full connectivity diagram.
