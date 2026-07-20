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

## MinerU CUDA route

The high-detail structured OCR route uses MinerU 3.4.4 in the local isolated
environment `$HOME/.local/share/4-bit/mineru-3.4.4/venv`. The environment uses
Python 3.12, PyTorch 2.11.0 with CUDA 13.0, and the NVIDIA GeForce RTX 4070 Ti
at compute capability 8.9. Its model corpus resides outside the repository at
`$HOME/.local/share/4-bit/mineru-3.4.4/models`; the ignored output cache
records the executable, model source, CUDA device, capability, free VRAM, and
every output digest.

`scripts/extract_mod40_mineru_ocr.sh` selects one PDF sheet per MinerU request,
forces OCR mode, uses the local `hybrid-engine` backend with high effort, and
serializes API work. It bounds rendering to four threads and processing to an
eight-page window. One-sheet requests make an output, a log, and a failure
state attributable to one exact source page rather than to a 35-page batch.

MinerU 3.4.4 rejects `auto` when it is inherited as the model-download source.
The local model download therefore uses the explicit Hugging Face source, and
the runtime records `MINERU_MODEL_SOURCE=huggingface`. This is a local tool
configuration boundary, not a claim about any schematic content.

Run the highest-priority route sheets with:

```sh
scripts/extract_mod40_mineru_ocr.sh --pages 3,5,7,10,13,29
```

Run all sheets only after a reviewed priority capture succeeds:

```sh
scripts/extract_mod40_mineru_ocr.sh --all-pages
```

MinerU Markdown, layout JSON, and recognized labels remain candidate material.
They can identify a crop or an endpoint name, but they never establish a wire,
an inversion, a Boolean function, or a pulse width without visual tracing on
the original sheet.

The first CPU-card comparison establishes a narrower tool result. MinerU's
`hybrid-engine` classifies the full PDF 3 schematic as one flowchart image and
recovers only its title block. Its `pipeline` backend classifies both the full
sheet and a direct 1200 DPI clock/reset PNG crop as one image. MinerU therefore
remains a page-layout, title-block, and artifact-normalization tool for this
source family; it does not provide the circuit-label decomposition path.

`scripts/extract_mod40_png_regions.py` supplies that path. It registers each
existing high-value JPEG review crop against the 600 DPI source render, reruns
the identical rectangle directly from the retained PDF as a lossless 1200 DPI
PNG, and records the registration score and source/output digests. Tesseract
PSM 11 and RapidOCR then emit independent label candidates with coordinates.
The PNG and candidates remain ignored, regeneratable review material. The
renderer requires a registration score of at least 0.90. It records a weaker
legacy-crop match as rejected and does not render a source PNG from it; a
manual source-coordinate recovery remains required.

The direct-PNG review queue covers the CPU card, motherboard/control boundary,
IN-28, panel logic, and TTY reference sheets. `page-05-controller-p1.jpg`
registers at 0.501327 and remains rejected. The remaining selected review
regions register at or above 0.977137. RapidOCR recovers useful labels such as
`CPU RESET`, `5.185MHz`, `A32`, `74158`, `4289`, `BYTE1`, `BYTE2`, `WRITE`,
`STOP ACK`, `TTY IN`, P4/J4 contacts, and the ASR-33 terminal-strip labels.
These labels agree with existing review targets but do not close a route gate.

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
