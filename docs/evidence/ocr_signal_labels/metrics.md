# OCR signal label verification metrics

Generated from `docs/evidence/ocr_signal_labels/*/*_signal_ocr_report.json`.

| Chip | Label points | OK | Mismatch | OK rate | Top mismatch reasons |
|------|-------------:|---:|---------:|--------:|----------------------|
| 4001 | 31 | 4 | 27 | 0.129 | ocr_no_tokens:26; ocr_low_conf:1 |
| 4002 | 50 | 1 | 49 | 0.020 | ocr_no_tokens:48; ocr_low_conf:1 |
| 4003 | 14 | 0 | 14 | 0.000 | ocr_no_tokens:12; ocr_low_conf:2 |
| 4004 | 50 | 0 | 50 | 0.000 | ocr_no_tokens:50 |

## Notes

- High mismatch rates are expected because many `signals.txt` entries are net identifiers, while the schematic prints pin numbers or local labels near the sampled coordinate.
- The `not_printed_near_point` reason is the working hypothesis when OCR finds confident nearby tokens but none resemble the expected net name.
