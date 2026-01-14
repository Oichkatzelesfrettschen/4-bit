# 4004 pad→pin mapping template (v0)

This file is intended to be human-edited.

- Pad detections: `docs/evidence/layout_pads_v0/4004/4004_layout_pads_v0.json`
- Edge labels: `docs/evidence/layout_edge_labels_v0/4004/4004_layout_edge_labels_v0.json`

Fill `pin_dip` and `signal` using the primary-source pinout diagrams, then map to anchors.

| pad_idx | edge | bbox (x0,y0,x1,y1) | suggested_node | edge_label | pin_dip | signal | confidence | notes |
|---:|---|---|---:|---|---:|---|---:|---|
| 0 | top | (172,39,317,125) | 0 | T | 10 | TEST | 0.85 | AUTO_EDGE_TOKEN |
| 0 | top | (172,39,317,125) | 0 | T | 10 | TEST_PAD | 0.85 | Alias of `TEST` (external pad anchor). |
| 1 | top | (549,40,689,126) | 1 | C | 9 | POC_PAD | 0.80 | AUTO_EDGE_TOKEN; `POC_PAD` is treated as external reset (`POC`) in emulator docs |
| 1 | top | (549,40,689,126) | 1 | C | 9 | RESET | 0.80 | Alias of `POC_PAD` for primary-source pin naming |
| 1 | top | (549,40,689,126) | 1 | C | 9 | POC | 0.80 | Alias of `POC_PAD` (internal net name used in some schematics). |
| 2 | top | (982,41,1108,132) | 2 | S | 8 | SYNC | 0.80 | AUTO_EDGE_TOKEN |
| 3 | top | (1225,42,1314,146) | 3 | G |  |  |  |  |
| 4 | top | (1392,50,1457,115) | 9 | G |  |  |  |  |
| 5 | top | (1555,43,1675,128) | 4 | G |  |  |  |  |
| 6 | right | (1815,266,1942,383) | 49 | G |  |  |  |  |
| 7 | right | (1856,1906,1942,2024) | 432 | G |  |  |  |  |
| 8 | right | (1853,2372,1938,2490) | 514 | G |  |  |  |  |
| 9 | bottom | (1227,2581,1340,2665) | 593 | G |  |  |  |  |
| 10 | bottom | (743,2579,870,2664) | 592 | G |  |  |  |  |
| 11 | bottom | (472,2578,605,2664) | 596 | G |  |  |  |  |
| 12 | bottom | (151,2577,284,2663) | 595 | G |  |  |  |  |
| 13 | left | (26,2086,112,2228) | 441 | R3 | 16 | CMRAM3 | 0.75 | AUTO_EDGE_TOKEN |
| 14 | left | (26,1785,112,1914) | 441 | R3 | 16 | CMRAM3 | 0.75 | AUTO_EDGE_TOKEN |
| 15 | left | (27,367,121,481) | 71 | RM | 11 | CMROM | 0.75 | AUTO_EDGE_TOKEN |
