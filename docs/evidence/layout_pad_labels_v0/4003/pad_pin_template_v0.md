# 4003 pad→pin mapping template (v0)

This file is intended to be human-edited.

- Pad detections: `docs/evidence/layout_pads_v0/4003/4003_layout_pads_v0.json`
- Edge labels: `docs/evidence/layout_edge_labels_v0/4003/4003_layout_edge_labels_v0.json`

Fill `pin_dip` and `signal` using the primary-source pinout diagrams, then map to anchors.

| pad_idx | edge | bbox (x0,y0,x1,y1) | suggested_node | edge_label | pin_dip | signal | confidence | notes |
|---:|---|---|---:|---|---:|---|---:|---|
| 0 | top | (28,26,261,177) | 0 |  | 1 | CLOCK | 0.3 | CP in primary source; mapped to `CLOCK` anchor. |
| 1 | top | (466,26,613,252) | 1 |  | 2 | DATA | 0.35 | Use alt suggested node `1` to avoid sharing node `0` with pad0. |
| 2 | top | (1232,26,1381,241) | 3 |  | 3 | Q0 | 0.3 | O0 in primary source; mapped to `Q0` anchor. |
| 3 | top | (1536,27,1686,239) | 4 |  | 4 | Q1 | 0.3 | O1 in primary source; mapped to `Q1` anchor. |
| 4 | right | (2150,27,2298,216) | 5 |  | 5 | VSS | 0.3 | Hypothesis: pad order follows DIP pin order (CCW). |
| 5 | right | (2150,552,2298,762) | 44 |  | 6 | Q2 | 0.3 | O2 in primary source; mapped to `Q2` anchor. |
| 6 | right | (2084,911,2298,1061) | 98 |  | 7 | Q3 | 0.3 | O3 in primary source; mapped to `Q3` anchor. |
| 7 | right | (2149,1211,2298,1413) | 122 |  | 8 | Q4 | 0.3 | O4 in primary source; mapped to `Q4` anchor. |
| 8 | bottom | (1852,1298,2105,1456) | 138 |  | 9 | Q5 | 0.3 | O5 in primary source; mapped to `Q5` anchor. |
| 9 | bottom | (1736,1201,1795,1317) | 7 |  | 10 | Q6 | 0.3 | O6 in primary source; mapped to `Q6` anchor. |
| 10 | bottom | (1232,1261,1456,1412) | 132 |  | 11 | Q7 | 0.3 | O7 in primary source; mapped to `Q7` anchor. |
| 11 | bottom | (936,1210,1084,1412) | 130 |  | 12 | Q8 | 0.35 | Use alt suggested node `130` to disambiguate from pad10’s node `131`. |
| 12 | bottom | (416,1204,565,1412) | 129 |  | 13 | Q9 | 0.3 | O9 in primary source; mapped to `Q9` anchor. |
| 13 | bottom | (28,1261,266,1412) | 129 |  | 14 | VDD | 0.3 | VDD on primary source diagram (pin 14). |
| 14 | left | (28,994,229,1187) | 109 |  | 15 | OUT | 0.3 | Serial out in primary source; mapped to `OUT` anchor. |
| 15 | left | (28,295,241,476) | 19 |  | 16 | EN | 0.3 | E (enable) in primary source; mapped to `EN` anchor. |
