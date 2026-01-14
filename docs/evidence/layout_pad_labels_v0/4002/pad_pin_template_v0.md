# 4002 pad→pin mapping template (v0)

This file is intended to be human-edited.

- Pad detections: `docs/evidence/layout_pads_v0/4002/4002_layout_pads_v0.json`
- Edge labels: `docs/evidence/layout_edge_labels_v0/4002/4002_layout_edge_labels_v0.json`

Fill `pin_dip` and `signal` using the primary-source pinout diagrams, then map to anchors.

| pad_idx | edge | bbox (x0,y0,x1,y1) | suggested_node | edge_label | pin_dip | signal | confidence | notes |
|---:|---|---|---:|---|---:|---|---:|---|
| 0 | top | (589,55,747,151) | 0 |  | 1 | D0_PAD | 0.3 | Hypothesis: pad order follows DIP pin order (CCW). |
| 1 | top | (885,55,1027,151) | 1 |  | 2 | D1_PAD | 0.3 | Hypothesis: pad order follows DIP pin order (CCW). |
| 2 | top | (1459,55,1576,162) | 2 |  | 3 | D2_PAD | 0.3 | Hypothesis: pad order follows DIP pin order (CCW). |
| 3 | top | (1819,56,1935,152) | 3 |  | 4 | D3_PAD | 0.3 | Hypothesis: pad order follows DIP pin order (CCW). |
| 4 | right | (2032,588,2135,703) | 128 |  | 5 | VSS | 0.3 | Hypothesis mapping; refine with better evidence. |
| 5 | right | (2033,944,2135,1061) | 292 |  | 6 | CLK1 | 0.3 | Hypothesis mapping; refine with better evidence. |
| 6 | right | (2042,1364,2135,1478) | 536 |  | 7 | CLK2 | 0.3 | Hypothesis mapping; refine with better evidence. |
| 7 | bottom | (1639,2421,1756,2518) | 819 |  | 8 | SYNC | 0.3 | Hypothesis mapping; refine with better evidence. |
| 8 | bottom | (1233,2421,1349,2518) | 818 |  | 9 | RESET | 0.3 | Hypothesis mapping; refine with better evidence. |
| 9 | bottom | (993,2421,1108,2518) | 817 |  | 10 | CS | 0.3 | Pin 10 is P0/Po in manual; mapped to `CS` anchor. |
| 10 | bottom | (695,2402,793,2517) | 4 |  | 11 | CM | 0.3 | `CM` is CM-RAM from CPU. |
| 11 | bottom | (403,2420,518,2517) | 816 |  | 12 | VDD | 0.3 | Hypothesis mapping; refine with better evidence. |
| 12 | left | (43,2007,141,2122) | 738 |  | 13 | OUT0 | 0.3 | Output port bit naming per `docs/emulators/i4002-signals.txt`. |
| 13 | left | (3,1803,124,1926) | 672 |  | 14 | OUT1 | 0.3 | Output port bit naming per `docs/emulators/i4002-signals.txt`. |
| 14 | left | (43,1308,141,1449) | 320 |  | 15 | OUT2 | 0.3 | Output port bit naming per `docs/emulators/i4002-signals.txt`. |
| 15 | left | (43,670,141,815) | 41 |  | 16 | OUT3 | 0.3 | Output port bit naming per `docs/emulators/i4002-signals.txt`. |
