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

## 4001/4002/4003 schematic bus labels
- Sources: docs/emulators/i4001-schematic.bmp, docs/emulators/i4002-schematic.bmp, docs/emulators/i4003-schematic.bmp
- Signal maps: docs/emulators/i4001-signals.txt, docs/emulators/i4002-signals.txt, docs/emulators/i4003-signals.txt
- Outputs:
  - docs/4001/annotated/i4001-schematic-bus-labels.png
  - docs/4002/annotated/i4002-schematic-bus-labels.png
  - docs/4003/annotated/i4003-schematic-bus-labels.png

Method
- Labeled clock/reset/bus pads from the analyzer signal maps.
- 4003 labels include CLOCK/DATA/EN/OUT and Q0-Q9 taps.

Notes
- Label selection favors bus pins and latch outputs to avoid clutter.

## 4001/4002/4003 transistor overlays (poly + diffusion)
- Sources: docs/emulators/i4001-poly.bmp, docs/emulators/i4002-poly.bmp, docs/emulators/i4003-poly.bmp
- Sources: docs/emulators/i4001-diffusion.bmp, docs/emulators/i4002-diffusion.bmp, docs/emulators/i4003-diffusion.bmp
- Outputs:
  - docs/4001/annotated/i4001-poly-diffusion-transistors.png
  - docs/4002/annotated/i4002-poly-diffusion-transistors.png
  - docs/4003/annotated/i4003-poly-diffusion-transistors.png

Method
- Same algorithm as the 4004 overlay: threshold masks, color poly/diffusion, highlight intersections.

Notes
- These overlays are for quick transistor-site visualization only.
