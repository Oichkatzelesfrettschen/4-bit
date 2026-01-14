# 4001 pad→pin mapping template (v0)

This file is intended to be human-edited.

- Pad detections: `docs/evidence/layout_pads_v0/4001/4001_layout_pads_v0.json`
- Edge labels: `docs/evidence/layout_edge_labels_v0/4001/4001_layout_edge_labels_v0.json`

Fill `pin_dip` and `signal` using the primary-source pinout diagrams, then map to anchors.

## Primary-source DIP pinout (reference)

From `docs/evidence/PRIMARY_SOURCE_PINOUTS.md` (MCS-40 User's Manual, Figure 4-5):

| DIP pin | signal |
|---:|---|
| 1 | D0 |
| 2 | D1 |
| 3 | D2 |
| 4 | D3 |
| 5 | VSS |
| 6 | φ1 (`CLK1`) |
| 7 | φ2 (`CLK2`) |
| 8 | SYNC |
| 9 | RESET |
| 10 | CL |
| 11 | CM-ROM (`CM`) |
| 12 | VDD |
| 13 | I/O0 (`IO0`) |
| 14 | I/O1 (`IO1`) |
| 15 | I/O2 (`IO2`) |
| 16 | I/O3 (`IO3`) |

## Known layout tokens (weak)

Edge-label OCR on the metal mask is sparse for 4001. Current hints:

- The cyclic angle-alignment seed suggestions in `docs/evidence/anchor_seed_suggestions_v0/4001_angle_alignment.json` indicate `CLK1→node 2540` and `CLK2→node 2522`.
- `pad_idx=14` has edge token `C`, but the current `CM` seed is `node 2529` (angle-alignment), so this pad is **unverified**.
- `pad_idx=15` previously produced `V` in some runs; this is not stable and should be treated as **untrusted**.

| pad_idx | edge | bbox (x0,y0,x1,y1) | suggested_node | edge_label | pin_dip | signal | confidence | notes |
|---:|---|---|---:|---|---:|---|---:|---|
| 0 | top | (194,60,333,149) | 0 |  | 1 | D0_PAD | 0.3 | Hypothesis: pad order follows DIP pin order (CCW). |
| 1 | top | (636,60,774,154) | 1 |  | 2 | D1_PAD | 0.3 | Hypothesis: pad order follows DIP pin order (CCW). |
| 2 | top | (1100,89,1246,186) | 3 |  | 3 | D2_PAD | 0.3 | Hypothesis: pad order follows DIP pin order (CCW). |
| 3 | top | (1587,89,1735,178) | 4 |  | 4 | D3_PAD | 0.3 | Hypothesis: pad order follows DIP pin order (CCW). |
| 4 | top | (2097,90,2248,179) | 4 |  | 5 | VSS | 0.3 | Hypothesis mapping; refine with better evidence. |
| 5 | right | (2566,296,2652,419) | 13 |  | 6 | CLK1 | 0.3 | Hypothesis mapping; refine with better evidence. |
| 6 | right | (2566,649,2653,761) | 14 |  | 7 | CLK2 | 0.3 | Hypothesis mapping; refine with better evidence. |
| 7 | right | (2565,1135,2651,1270) | 1536 |  | 8 | SYNC | 0.3 | Hypothesis mapping; refine with better evidence. |
| 8 | right | (2555,1436,2640,1592) | 2514 |  | 9 | RESET | 0.3 | Hypothesis mapping; refine with better evidence. |
| 9 | bottom | (2179,1659,2425,1768) | 2540 | S | 10 | CL | 0.3 | Edge-label token `S` is ambiguous on 4001 and not treated as `SYNC`. |
| 10 | bottom | (1995,1587,2097,1708) | 2522 |  | 11 | CM | 0.3 | CM-ROM in the manual; mapped to `CM` anchor. |
| 11 | bottom | (1620,1584,1723,1708) | 2523 |  | 12 | VDD | 0.3 | Hypothesis mapping; refine with better evidence. |
| 12 | bottom | (770,1620,935,1708) | 2539 |  | 13 | IO0 | 0.3 | Hypothesis mapping; refine with better evidence. |
| 13 | bottom | (435,1620,582,1709) | 2538 |  | 14 | IO1 | 0.3 | Hypothesis mapping; refine with better evidence. |
| 14 | left | (62,1483,159,1603) | 2519 | C | 15 | IO2 | 0.3 | Edge-label token `C` is ambiguous on 4001 (not treated as `CM`). |
| 15 | left | (105,405,191,567) | 111 | V | 16 | IO3 | 0.3 | Edge-label token `V` may be supply-related; kept as untrusted. |
