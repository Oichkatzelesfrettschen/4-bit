#!/bin/sh
# Extract page-isolated MOD 40 OCR candidates with a pinned CUDA MinerU install.
set -eu

REPO_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
SOURCE_PDF="$REPO_ROOT/docs/MCS-40/Intel_Intellec_4_MOD_40_Reference_Schematics.pdf"
MINERU_HOME=${MINERU_HOME:-"$HOME/.local/share/4-bit/mineru-3.4.4"}
MINERU_EXEC=${MINERU_EXEC:-"$MINERU_HOME/venv/bin/mineru"}
OUTPUT_DIRECTORY=${OUTPUT_DIRECTORY:-"$REPO_ROOT/docs/evidence/ocr/mod40_98013_20260713/mineru-3.4.4"}
PAGE_LIST=3,5,7,10,13,29
BACKEND=hybrid-engine
EFFORT=high

usage() {
    printf '%s\n' \
        'Usage: scripts/extract_mod40_mineru_ocr.sh [--output DIRECTORY] [--pages LIST] [--all-pages] [--backend NAME] [--effort NAME]' \
        'Runs page-isolated MinerU OCR against the retained 35-page MOD 40 schematic scan.' \
        'LIST uses one-based PDF pages, for example: 3,5,7,10,13,29.'
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --output)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            OUTPUT_DIRECTORY=$2
            shift 2
            ;;
        --pages)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            PAGE_LIST=$2
            shift 2
            ;;
        --all-pages)
            PAGE_LIST=$(seq -s, 1 35)
            shift
            ;;
        --backend)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            BACKEND=$2
            shift 2
            ;;
        --effort)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            EFFORT=$2
            shift 2
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            usage >&2
            exit 2
            ;;
    esac
done

for required_command in pdfinfo sha256sum nvidia-smi sort find xargs; do
    command -v "$required_command" >/dev/null 2>&1 || {
        printf 'missing required command: %s\n' "$required_command" >&2
        exit 127
    }
done

[ -f "$SOURCE_PDF" ] || {
    printf 'missing source PDF: %s\n' "$SOURCE_PDF" >&2
    exit 1
}
[ -x "$MINERU_EXEC" ] || {
    printf 'MinerU executable is not executable: %s\n' "$MINERU_EXEC" >&2
    exit 1
}

page_count=$(pdfinfo "$SOURCE_PDF" | awk '/^Pages:/ { print $2 }')
[ "$page_count" = 35 ] || {
    printf 'expected 35 schematic pages, found: %s\n' "${page_count:-unknown}" >&2
    exit 1
}

case "$BACKEND" in
    pipeline|vlm-engine|hybrid-engine|vlm-http-client|hybrid-http-client) ;;
    *)
        printf 'unsupported MinerU backend: %s\n' "$BACKEND" >&2
        exit 2
        ;;
esac
case "$EFFORT" in
    medium|high) ;;
    *)
        printf 'unsupported MinerU effort: %s\n' "$EFFORT" >&2
        exit 2
        ;;
esac

mkdir -p "$OUTPUT_DIRECTORY/metadata" "$OUTPUT_DIRECTORY/logs" "$OUTPUT_DIRECTORY/pages"

export CUDA_VISIBLE_DEVICES=${CUDA_VISIBLE_DEVICES:-0}
export HF_HOME=${HF_HOME:-"$MINERU_HOME/models/huggingface"}
export MODELSCOPE_CACHE=${MODELSCOPE_CACHE:-"$MINERU_HOME/models/modelscope"}
export MINERU_MODEL_SOURCE=${MINERU_MODEL_SOURCE:-huggingface}
export MINERU_PDF_RENDER_TIMEOUT=${MINERU_PDF_RENDER_TIMEOUT:-900}
export MINERU_PDF_RENDER_THREADS=${MINERU_PDF_RENDER_THREADS:-4}
export MINERU_PROCESSING_WINDOW_SIZE=${MINERU_PROCESSING_WINDOW_SIZE:-8}
export MINERU_API_MAX_CONCURRENT_REQUESTS=${MINERU_API_MAX_CONCURRENT_REQUESTS:-1}
export MINERU_LOCAL_API_STARTUP_TIMEOUT_SECONDS=${MINERU_LOCAL_API_STARTUP_TIMEOUT_SECONDS:-900}
export MINERU_TASK_RESULT_TIMEOUT_SECONDS=${MINERU_TASK_RESULT_TIMEOUT_SECONDS:-5400}
export PYTORCH_CUDA_ALLOC_CONF=${PYTORCH_CUDA_ALLOC_CONF:-expandable_segments:True}

{
    printf 'source_pdf=%s\n' "$SOURCE_PDF"
    printf 'source_sha256=%s\n' "$(sha256sum "$SOURCE_PDF" | awk '{ print $1 }')"
    printf 'page_count=%s\n' "$page_count"
    printf 'mineru_exec=%s\n' "$MINERU_EXEC"
    printf 'mineru_version=%s\n' "$("$MINERU_EXEC" --version)"
    printf 'method=ocr\n'
    printf 'backend=%s\n' "$BACKEND"
    printf 'effort=%s\n' "$EFFORT"
    printf 'pages_one_based=%s\n' "$PAGE_LIST"
    printf 'model_source=%s\n' "$MINERU_MODEL_SOURCE"
    printf 'hf_home=%s\n' "$HF_HOME"
    printf 'cuda_visible_devices=%s\n' "$CUDA_VISIBLE_DEVICES"
    printf 'mineru_pdf_render_timeout=%s\n' "$MINERU_PDF_RENDER_TIMEOUT"
    printf 'mineru_pdf_render_threads=%s\n' "$MINERU_PDF_RENDER_THREADS"
    printf 'mineru_processing_window_size=%s\n' "$MINERU_PROCESSING_WINDOW_SIZE"
    printf 'mineru_api_max_concurrent_requests=%s\n' "$MINERU_API_MAX_CONCURRENT_REQUESTS"
    printf 'pytorch_cuda_alloc_conf=%s\n' "$PYTORCH_CUDA_ALLOC_CONF"
    "$MINERU_EXEC" --version
    "$MINERU_EXEC" --help | awk '/^-m|^  -m|^-b|^  -b|--effort/ { print }'
    nvidia-smi --query-gpu=name,driver_version,memory.total,memory.free --format=csv,noheader
    "$MINERU_HOME/venv/bin/python" - <<'PY'
import importlib.metadata
import torch

print(f"python={__import__('sys').version.split()[0]}")
print(f"mineru_distribution={importlib.metadata.version('mineru')}")
print(f"torch={torch.__version__}")
print(f"torch_cuda={torch.version.cuda}")
print(f"cuda_available={torch.cuda.is_available()}")
if torch.cuda.is_available():
    print(f"cuda_device={torch.cuda.get_device_name(0)}")
    print(f"cuda_capability={torch.cuda.get_device_capability(0)}")
PY
} >"$OUTPUT_DIRECTORY/metadata/provenance.txt"

old_ifs=$IFS
IFS=,
set -- $PAGE_LIST
IFS=$old_ifs
[ "$#" -gt 0 ] || {
    printf '%s\n' 'page list is empty' >&2
    exit 2
}

for page_number in "$@"; do
    case "$page_number" in
        ''|*[!0-9]*)
            printf 'invalid one-based page number: %s\n' "$page_number" >&2
            exit 2
            ;;
    esac
    [ "$page_number" -ge 1 ] && [ "$page_number" -le "$page_count" ] || {
        printf 'page outside 1..%s: %s\n' "$page_count" "$page_number" >&2
        exit 2
    }

    page_name=$(printf 'page-%02d' "$page_number")
    page_start=$((page_number - 1))
    page_directory="$OUTPUT_DIRECTORY/pages/$page_name"
    page_status="$OUTPUT_DIRECTORY/logs/$page_name.status"

    if [ -f "$page_status" ] && grep -qx 'result=success' "$page_status"; then
        printf 'already complete: %s\n' "$page_name"
        continue
    fi

    mkdir -p "$page_directory"
    {
        printf 'page_one_based=%s\n' "$page_number"
        printf 'mineru_start_zero_based=%s\n' "$page_start"
        printf 'mineru_end_zero_based=%s\n' "$page_start"
        printf 'result=running\n'
    } >"$page_status"

    set +e
    "$MINERU_EXEC" -p "$SOURCE_PDF" -o "$page_directory" -m ocr -b "$BACKEND" \
        --effort "$EFFORT" -s "$page_start" -e "$page_start" \
        --client-side-output-generation true \
        >"$OUTPUT_DIRECTORY/logs/$page_name.stdout" \
        2>"$OUTPUT_DIRECTORY/logs/$page_name.stderr"
    mineru_status=$?
    set -e

    if [ "$mineru_status" -ne 0 ]; then
        {
            printf 'page_one_based=%s\n' "$page_number"
            printf 'mineru_start_zero_based=%s\n' "$page_start"
            printf 'mineru_end_zero_based=%s\n' "$page_start"
            printf 'result=failure\n'
            printf 'exit_status=%s\n' "$mineru_status"
        } >"$page_status"
        printf 'MinerU failed on %s; inspect: %s\n' "$page_name" \
            "$OUTPUT_DIRECTORY/logs/$page_name.stderr" >&2
        exit "$mineru_status"
    fi

    {
        printf 'page_one_based=%s\n' "$page_number"
        printf 'mineru_start_zero_based=%s\n' "$page_start"
        printf 'mineru_end_zero_based=%s\n' "$page_start"
        printf 'result=success\n'
        printf 'exit_status=0\n'
    } >"$page_status"
done

(
    cd "$OUTPUT_DIRECTORY"
    find metadata logs pages -type f ! -path 'metadata/artifacts.sha256' -print0 \
        | sort -z | xargs -0 sha256sum
) >"$OUTPUT_DIRECTORY/metadata/artifacts.sha256"

printf 'MinerU OCR extraction complete: %s\n' "$OUTPUT_DIRECTORY"
