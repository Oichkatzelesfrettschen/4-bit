# Intellec 4 MOD 40 98-013A OCR Status

## Evidence boundary

OCR discovers page-scoped candidate labels. It does not establish a wire, a
polarity, a connector contact, a Boolean function, or a timing relationship.
Only a visually reviewed primary-sheet trace, with both endpoints and every
inverting stage recorded, can update
`INTELLEC_MOD40_BOARD_NET_LEDGER.md`.

## Full-sheet method

`scripts/extract_mod40_schematic_ocr.sh` renders all 35 sheets from the retained
98-013A scan with Poppler at 600 DPI and JPEG quality 95. It retains source
SHA-256, engine versions, parameters, and per-artifact SHA-256 values in the
generated local evidence cache. The full-sheet cache remains ignored because
the 600 DPI image corpus is large and mechanically reproducible from the
source scan.

The capture runs three independent recognition paths:

| Engine | Input and configuration | Role | Result boundary |
| --- | --- | --- | --- |
| Tesseract 5.5.2 | 600 DPI JPEG, PSM 6 and PSM 11 | Primary text discovery; layout and sparse-label interpretations complement each other. | Candidate text only. |
| GNU OCRad 0.29 | Independent 300 DPI grayscale PNM render | Independent low-complexity comparison. PNM avoids OCRad's unsupported JPEG input. | Candidate text only; materially weaker on dense drawings. |
| Surya OCR 0.17 | 600 DPI JPEG; CUDA recognition batch 32 | GPU-assisted dense-label discovery on central processor, motherboard, controller, IN-28, panel, display, and TTY sheets. | Candidate text only; a source-sheet review remains mandatory. |

The NVIDIA GeForce RTX 4070 Ti provides 12282 MiB of VRAM. The installed
CUDA-enabled PyTorch runtime reports CUDA 13.3 and compute capability 8.9.
Surya's default recognition batch of 256 exhausts VRAM on dense sheets. The
pipeline uses `RECOGNITION_BATCH_SIZE=32` and
`PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True`; this completes the selected
sheet set without changing recognition semantics.

The system package `python-surya-ocr` installs successfully. Its global import
conflicts with the workstation's Transformer 5.2.0 and tokenizers 0.23.1
combination. The capture uses an isolated `uv` environment with Transformers
4.57.6, tokenizers 0.22.2, and Hugging Face Hub 0.36.2 while sharing the
system CUDA PyTorch runtime. The repository script accepts the isolated Python
path as an explicit argument and does not mutate global Python dependencies.

GOCR 0.52 remains useful for small diagnostic crops but does not complete one
600 DPI dense sheet in an acceptable interval. It is excluded from the full
corpus pass. OCRmyPDF, ScanTailor Advanced, OpenCV CUDA, and unpaper remain
available for preprocessing, but the source scan does not need destructive
binarization before the first review pass.

## Result

Tesseract resolves title text that the earlier 1800px review cannot resolve.
The independent OCRad path produces materially degraded topology labels. The
CUDA Surya path recovers additional dense labels, including `BYTE1`, `BYTE2`,
`MAD`, `MODULE SELECT`, and `WRITE` on the IN-28 sheet. The overlap provides
search and review targets, not net proof. Manual review of the 600 DPI renders
resolves residual title ambiguity and traces circuit connectivity.

The completed capture contains 35 review renders, 35 Tesseract PSM 6 attempt
records, 35 Tesseract PSM 11 attempt records, 35 OCRad records, and seven
Surya page results. Both Tesseract modes time out at the recorded 60-second
limit on PDF page 35, the content-free back cover. The remaining 34 sheets
complete both Tesseract layouts. The candidate index contains 50 normalized
page-scoped labels. Five labels occur in two independent engines: CPU on page
5, STOP ACK on page 7, and BYTE2, MAD, and WRITE on page 10. These are review
priorities, not net claims.

ImageMagick 7.1.2-26 rejects Poppler PNG output with `bad adaptive filter
value`; it reads the JPEG render correctly. The full-sheet pipeline therefore
uses JPEG and records the decoder limitation rather than treating an
image-decoder error as OCR evidence.

## Targeted route extraction

Endpoint review uses Poppler direct JPEG crops at 1200 DPI. A full-sheet 1200
DPI render exceeds the workstation ImageMagick pixel-cache limit, while
Poppler direct cropping keeps the primary source pixels intact and bounds each
review unit. The page 5 motherboard connector labels use an 1800 DPI crop
where the 1200 DPI labels remain ambiguous.

The completed targeted review covers the motherboard, memory-controller, and
IN-28 program-RAM boundary on PDFs 5, 7, and 10. It identifies the low address
contacts 11 through 20, high address contacts 94 and 96, and the byte, module-
select, and write contacts. The route facts and their remaining timing limits
are recorded only in `INTELLEC_MOD40_BOARD_NET_LEDGER.md`.

The Intel 1975 Data Catalog is independently rendered at 1200 DPI for the
2102 and 3404 device contracts. The 2102 write-cycle waveform retains data at
the rising edge that ends an active-low `R/W` pulse. The 3404 data sheet
defines two active-low, transparent, inverting latch groups. These component
facts correct model behavior but do not complete an IN-28 board-cycle claim.

The table records a title result only; it never authorizes board wiring.

| PDF page | Tesseract title result | Status |
| --- | --- | --- |
| 2 | Printed Wiring Assembly, Central Processor Module | title resolved; nets open |
| 4 | Printed Wiring Assembly, Mother Board | title resolved; nets open |
| 5 | Schematic, Mother Board; drawing 2000077 | title and drawing resolved; nets open |
| 6 | Memory Controller Assembly | title resolved; nets open |
| 10 | Schematic, IN-28; drawing appears 01-0176-001 | title resolved; drawing requires manual digit confirmation |
| 12 | Printed Wiring Assembly, Front Panel Logic, sheet 2 | title resolved; nets open |
| 14 | Printed Wiring Assembly, Front Panel Display Board | title resolved; nets open |
| 25 | Data Storage/OEM Module schematic; drawing 2000121 | title and drawing resolved by manual review; nets open |
| 28 | Power One CP110, D5-12/S113 power supplies schematic | title resolved; Intellec connector nets open |
| 35 | Back cover | no board content |

## Engine selection

Tesseract remains the reproducible CPU baseline. Surya is the strongest
installed GPU recognizer for this work because it couples text recognition with
layout detection and uses the installed CUDA PyTorch runtime. PaddleOCR and
EasyOCR remain available AUR alternatives, but neither is installed because a
second GPU stack does not replace the required visual endpoint and polarity
review. `python-doctr` is installed but its current global import is blocked by
the same Transformers/tokenizers dependency mismatch; it is not used as an
unverified fallback.

## Next extraction units

1. Crop page 4 and page 5 connector regions and reconcile every J1-J18 net.
2. Crop page 6 and page 7 matching controller connectors and reconcile mode,
   reset, monitor, and program-RAM selection nets.
3. Crop page 10 by bank and lane to capture MAD0-MAD11, byte, module-select,
   and inverted write paths.
4. Use targeted high-resolution crops and manual trace following for dense
   connector and polarity regions; do not infer them from OCR text.
5. Mark a net as source-backed only after its two endpoints and sheet locations
   are recorded in the board net ledger.
