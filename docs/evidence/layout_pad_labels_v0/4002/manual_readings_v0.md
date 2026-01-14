# 4002 metal-mask pad label readings (manual, v0)

This file is a human-maintained index for converting metal-mask pad-label crops into anchors.
- Canonical crops: `docs/evidence/layout_pad_labels_v0/4002/human_crops/`

Important:

- On **4002**, most detected “labels” are **die markings** (`4002`, logo fragments) or pad/block shapes rather than per-pin signal names.
- Treat `printed_label`/`ocr_best` as *hints* only; anchor assignment should be corroborated by primary-source pinouts (see `docs/evidence/PRIMARY_SOURCE_PINOUTS.md`) and the schematic-derived pin anchors in `docs/emulators/i4002-signals.txt`.

## Candidates

| idx | suggested_node | ocr_best | crop | printed_label | anchor_name | notes |
|---:|---:|---|---|---|---|---|
| 0 | 2 | `WL` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_000_node_2.png` | WL |  | AUTO_OCR_V1 conf=35.0 psm=8 inv=0 scale=2 |
| 1 | 3 | `1` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_001_node_3.png` | 1 |  | AUTO_OCR_V1 conf=64.0 psm=11 inv=0 scale=2 |
| 2 | 6 | `SALI` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_002_node_6.png` | SALI |  | AUTO_OCR_V1 conf=38.0 psm=7 inv=0 scale=2 |
| 3 | 7 | `[INL` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_003_node_7.png` | [INL |  | AUTO_OCR_V1 conf=62.0 psm=10 inv=0 scale=2 |
| 4 | 15 | `_ME` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_004_node_15.png` | _ME |  | AUTO_OCR_V1 conf=29.0 psm=11 inv=0 scale=3 |
| 5 | 16 | `UL` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_005_node_16.png` | UL |  | AUTO_OCR_V1 conf=70.0 psm=11 inv=0 scale=4 |
| 6 | 38 | `LED` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_006_node_38.png` | LED |  | AUTO_OCR_V1 conf=53.0 psm=8 inv=1 scale=3 |
| 7 | 72 | `24` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_007_node_72.png` | 24 |  | AUTO_OCR_V1 conf=46.0 psm=7 inv=1 scale=2 |
| 8 | 100 | `IL` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_008_node_100.png` | IL |  | AUTO_OCR_V1 conf=59.0 psm=8 inv=1 scale=2 |
| 9 | 128 | `F` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_009_node_128.png` | F | D2_PAD | seed from pad-like periphery node mapping (see anchors v0 note); AUTO_OCR_V1 conf=52.0 psm=10 inv=0 scale=3 |
| 10 | 237 | `I` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_010_node_237.png` | I |  | AUTO_OCR_V1 conf=42.0 psm=7 inv=1 scale=2 |
| 11 | 292 | `E` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_011_node_292.png` | E | D0_PAD | seed from pad-like periphery node mapping (see anchors v0 note); AUTO_OCR_V1 conf=87.0 psm=10 inv=0 scale=2 |
| 12 | 398 | `IL` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_012_node_398.png` | IL |  | AUTO_OCR_V1 conf=61.0 psm=8 inv=0 scale=4 |
| 13 | 559 | `[LL` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_013_node_559.png` | [LL |  | AUTO_OCR_V1 conf=49.0 psm=11 inv=1 scale=4 |
| 14 | 583 | `[X` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_014_node_583.png` | [X |  | AUTO_OCR_V1 conf=45.0 psm=8 inv=0 scale=2 |
| 15 | 606 | `EE` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_015_node_606.png` | EE |  | AUTO_OCR_V1 conf=68.0 psm=8 inv=0 scale=2 |
| 16 | 654 | `IE]` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_016_node_654.png` | IE] |  | AUTO_OCR_V1 conf=70.0 psm=8 inv=0 scale=2 |
| 17 | 672 | `AL` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_017_node_672.png` | AL | CS | seed from pad-like periphery node mapping (likely P0/CS pad); AUTO_OCR_V1 conf=78.0 psm=8 inv=0 scale=2 |
| 18 | 738 | `2` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_018_node_738.png` | 2 | CM | seed from pad-like periphery node mapping; AUTO_OCR_V1 conf=51.0 psm=8 inv=1 scale=2 |
| 19 | 655 | `[E-` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_019_node_655.png` | [E- |  | AUTO_OCR_V1 conf=36.5 psm=11 inv=0 scale=4 |
| 20 | 806 | `I` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_020_node_806.png` | I |  | AUTO_OCR_V1 conf=29.0 psm=7 inv=1 scale=2 |
| 21 | 815 | `OS` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_021_node_815.png` | OS |  | AUTO_OCR_V1 conf=54.0 psm=8 inv=0 scale=3 |
| 22 | 816 | `J)[RIN` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_022_node_816.png` | J)[RIN | RESET | seed from pad-like periphery node mapping; AUTO_OCR_V1 conf=43.0 psm=11 inv=0 scale=3 |
| 23 | 817 | `IM` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_023_node_817.png` | IM | OUT2 | seed from pad-like periphery node mapping; AUTO_OCR_V1 conf=77.0 psm=8 inv=0 scale=4 |
| 24 | 818 | `CG` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_024_node_818.png` | CG | OUT1 | seed from pad-like periphery node mapping; AUTO_OCR_V1 conf=77.0 psm=8 inv=1 scale=3 |
| 25 | 819 | `NGJ` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_025_node_819.png` | NGJ | OUT0 | seed from pad-like periphery node mapping; AUTO_OCR_V1 conf=55.0 psm=11 inv=0 scale=4 |
| 26 | 821 | `DOX7Z0NOGEXX/1INTTTTAL` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_026_node_821.png` | DOX7Z0NOGEXX/1INTTTTAL | OUT3 | seed from pad-like periphery node mapping; AUTO_OCR_V1 conf=40.8 psm=11 inv=0 scale=4 |
| 27 | 824 | `A0` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_027_node_824.png` | A0 |  | AUTO_OCR_V1 conf=92.0 psm=11 inv=0 scale=2 |
| 28 | 825 | `[4004` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_028_node_825.png` | [4004 |  | AUTO_OCR_V1 conf=58.0 psm=8 inv=1 scale=2 |
| 29 | 826 | `1002` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_029_node_826.png` | 1002 |  | AUTO_OCR_V1 conf=93.0 psm=11 inv=0 scale=2 |
| 30 | 827 | `02` | `docs/evidence/layout_pad_labels_v0/4002/human_crops/box_030_node_827.png` | 02 |  | AUTO_OCR_V1 conf=93.0 psm=11 inv=0 scale=3 |

## Derived pad anchors (no printable label)

Some pads have no adjacent printable label crop, but are still meaningful to anchor as `*_PAD` signals.
Current `_PAD` seeds are tracked in `docs/evidence/schematic_layout_anchors_v1.json` as `layout_node_src`
(pad-metal seed) and `layout_node` (remapped incident seed). Prefer that file as the source of truth.

Note: remapping to “incident” transistor nodes can collapse multiple external pads onto a shared internal
trunk net (e.g. `D0_PAD..D3_PAD`), so treat `layout_node` as a *subcircuit seed* and `layout_node_src`
as the *pad metal seed*.

## Current anchor mapping (v1)

Source of truth: `docs/evidence/schematic_layout_anchors_v1.json`.

Notes:

- `CS` is the repo’s canonical name for the primary-source pin `P0` / `Po` (chip selection metal-option input).
- `OUT0..OUT3` naming comes from `docs/emulators/i4002-signals.txt` and should be treated as **tentative** until corroborated by a primary diagram that numbers the 4-bit output port.

| signal | layout_node | layout_node_src |
|---|---:|---:|
| `CLK1` | 2259 | 292 |
| `CLK2` | 2619 | 536 |
| `SYNC` | 3261 | 819 |
| `RESET` | 3225 | 818 |
| `CS` | 789 | 817 |
| `CM` | 799 | 4 |
| `VDD` | 3251 | 816 |
| `VSS` | 73 | 128 |
| `D0_PAD` | 868 | 0 |
| `D1_PAD` | 872 | 1 |
| `D2_PAD` | 938 | 2 |
| `D3_PAD` | 896 | 3 |
| `OUT0` | 3115 | 738 |
| `OUT1` | 2855 | 672 |
| `OUT2` | 2605 | 320 |
| `OUT3` | 1685 | 41 |
