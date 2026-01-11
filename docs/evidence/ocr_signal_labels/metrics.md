# OCR signal label verification metrics

Generated from `docs/evidence/ocr_signal_labels/*/*_signal_ocr_report.json`.

| Chip | Label points | OK | Mismatch | OK rate | Top mismatch reasons |
|------|-------------:|---:|---------:|--------:|----------------------|
| 4001 | 63 | 4 | 59 | 0.063 | no_similar_token:29; ocr_low_conf:13; not_printed_near_point:10 |
| 4002 | 406 | 5 | 401 | 0.012 | ocr_low_conf:311; no_similar_token:57; not_printed_near_point:25 |
| 4003 | 14 | 0 | 14 | 0.000 | no_similar_token:8; not_printed_near_point:4; mismatch:2 |
| 4004 | 248 | 7 | 241 | 0.028 | ocr_low_conf:115; no_similar_token:73; not_printed_near_point:39 |

## Notes

- High mismatch rates are expected because many `signals.txt` entries are net identifiers, while the schematic prints pin numbers or local labels near the sampled coordinate.
- The `not_printed_near_point` reason is the working hypothesis when OCR finds confident nearby tokens but none resemble the expected net name.
