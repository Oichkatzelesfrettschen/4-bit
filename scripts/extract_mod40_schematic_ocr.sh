#!/bin/sh
# Render every 98-013A sheet and retain independent OCR discovery outputs.
set -eu

REPO_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
SOURCE_PDF="$REPO_ROOT/docs/MCS-40/Intel_Intellec_4_MOD_40_Reference_Schematics.pdf"
OUTPUT_DIRECTORY="$REPO_ROOT/docs/evidence/ocr/mod40_98013_20260713/full-sheet"
OCR_TIMEOUT_SECONDS=${OCR_TIMEOUT_SECONDS:-180}

usage() {
    printf '%s\n' \
        'Usage: scripts/extract_mod40_schematic_ocr.sh [--output DIRECTORY] [--surya-python PATH] [--surya-pages LIST] [--force]' \
        'Renders all 35 MOD 40 schematic sheets at 600 dpi and records OCR outputs.'
}

force=0
surya_python=
surya_pages=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --output)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            OUTPUT_DIRECTORY=$2
            shift 2
            ;;
        --surya-python)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            surya_python=$2
            shift 2
            ;;
        --surya-pages)
            [ "$#" -ge 2 ] || { usage >&2; exit 2; }
            surya_pages=$2
            shift 2
            ;;
        --force)
            force=1
            shift
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

for required_command in pdfinfo pdftoppm tesseract ocrad sha256sum timeout; do
    command -v "$required_command" >/dev/null 2>&1 || {
        printf 'missing required command: %s\n' "$required_command" >&2
        exit 127
    }
done

[ -f "$SOURCE_PDF" ] || {
    printf 'missing source PDF: %s\n' "$SOURCE_PDF" >&2
    exit 1
}

page_count=$(pdfinfo "$SOURCE_PDF" | awk '/^Pages:/ { print $2 }')
[ "$page_count" = 35 ] || {
    printf 'expected 35 schematic pages, found: %s\n' "${page_count:-unknown}" >&2
    exit 1
}

mkdir -p "$OUTPUT_DIRECTORY/render" \
    "$OUTPUT_DIRECTORY/tesseract-psm6" \
    "$OUTPUT_DIRECTORY/tesseract-psm11" \
    "$OUTPUT_DIRECTORY/ocrad-input" \
    "$OUTPUT_DIRECTORY/ocrad" \
    "$OUTPUT_DIRECTORY/metadata"

if [ -n "$surya_python" ]; then
    [ -x "$surya_python" ] || {
        printf 'Surya Python is not executable: %s\n' "$surya_python" >&2
        exit 1
    }
    [ -n "$surya_pages" ] || {
        printf '%s\n' 'Surya pages are required when Surya Python is specified' >&2
        exit 2
    }
    mkdir -p "$OUTPUT_DIRECTORY/surya"
fi

if [ "$force" -eq 1 ]; then
    rm -f "$OUTPUT_DIRECTORY"/render/* \
        "$OUTPUT_DIRECTORY"/tesseract-psm6/* \
        "$OUTPUT_DIRECTORY"/tesseract-psm11/* \
        "$OUTPUT_DIRECTORY"/ocrad-input/* \
        "$OUTPUT_DIRECTORY"/ocrad/* \
        "$OUTPUT_DIRECTORY"/metadata/*
    if [ -n "$surya_python" ]; then
        rm -rf "$OUTPUT_DIRECTORY/surya"
        mkdir -p "$OUTPUT_DIRECTORY/surya"
    fi
fi

{
    printf 'source_pdf=%s\n' "$SOURCE_PDF"
    printf 'source_sha256=%s\n' "$(sha256sum "$SOURCE_PDF" | awk '{ print $1 }')"
    printf 'page_count=%s\n' "$page_count"
    printf 'render_dpi=600\n'
    printf 'ocr_timeout_seconds=%s\n' "$OCR_TIMEOUT_SECONDS"
    printf 'tesseract=%s\n' "$(tesseract --version 2>&1 | awk 'NR == 1 { print }')"
    printf 'tesseract_psm=6,11\n'
    printf 'ocrad=%s\n' "$(ocrad --version 2>&1 | awk 'NR == 1 { print }')"
    printf 'ocrad_render_dpi=300\n'
    printf 'surya_python=%s\n' "${surya_python:-disabled}"
    printf 'surya_pages=%s\n' "${surya_pages:-disabled}"
    printf 'surya_recognition_batch=32\n'
    printf 'pytorch_cuda_alloc_conf=expandable_segments:True\n'
} >"$OUTPUT_DIRECTORY/metadata/provenance.txt"

page_number=1
while [ "$page_number" -le "$page_count" ]; do
    page_name=$(printf 'page-%02d' "$page_number")
    render_path="$OUTPUT_DIRECTORY/render/$page_name.jpg"

    if [ ! -f "$render_path" ]; then
        pdftoppm -f "$page_number" -l "$page_number" -r 600 -jpeg \
            -jpegopt quality=95,progressive=n -singlefile "$SOURCE_PDF" \
            "$OUTPUT_DIRECTORY/render/$page_name"
    fi

    if [ ! -s "$OUTPUT_DIRECTORY/tesseract-psm6/$page_name.txt" ] && \
        [ ! -f "$OUTPUT_DIRECTORY/tesseract-psm6/$page_name.status" ]; then
        set +e
        timeout "$OCR_TIMEOUT_SECONDS" tesseract "$render_path" \
            "$OUTPUT_DIRECTORY/tesseract-psm6/$page_name" --psm 6 \
            2>"$OUTPUT_DIRECTORY/tesseract-psm6/$page_name.stderr"
        tesseract_status=$?
        set -e
        if [ "$tesseract_status" -eq 124 ]; then
            printf 'result=timeout\ntimeout_seconds=%s\n' "$OCR_TIMEOUT_SECONDS" \
                >"$OUTPUT_DIRECTORY/tesseract-psm6/$page_name.status"
        elif [ "$tesseract_status" -ne 0 ]; then
            exit "$tesseract_status"
        elif [ ! -s "$OUTPUT_DIRECTORY/tesseract-psm6/$page_name.txt" ]; then
            printf 'result=no-text\n' >"$OUTPUT_DIRECTORY/tesseract-psm6/$page_name.status"
        fi
    fi
    if [ ! -s "$OUTPUT_DIRECTORY/tesseract-psm11/$page_name.txt" ] && \
        [ ! -f "$OUTPUT_DIRECTORY/tesseract-psm11/$page_name.status" ]; then
        set +e
        timeout "$OCR_TIMEOUT_SECONDS" tesseract "$render_path" \
            "$OUTPUT_DIRECTORY/tesseract-psm11/$page_name" --psm 11 \
            2>"$OUTPUT_DIRECTORY/tesseract-psm11/$page_name.stderr"
        tesseract_status=$?
        set -e
        if [ "$tesseract_status" -eq 124 ]; then
            printf 'result=timeout\ntimeout_seconds=%s\n' "$OCR_TIMEOUT_SECONDS" \
                >"$OUTPUT_DIRECTORY/tesseract-psm11/$page_name.status"
        elif [ "$tesseract_status" -ne 0 ]; then
            exit "$tesseract_status"
        elif [ ! -s "$OUTPUT_DIRECTORY/tesseract-psm11/$page_name.txt" ]; then
            printf 'result=no-text\n' >"$OUTPUT_DIRECTORY/tesseract-psm11/$page_name.status"
        fi
    fi
    if [ ! -f "$OUTPUT_DIRECTORY/ocrad/$page_name.txt" ]; then
        ocrad_input="$OUTPUT_DIRECTORY/ocrad-input/$page_name.pgm"
        if [ ! -f "$ocrad_input" ]; then
            pdftoppm -f "$page_number" -l "$page_number" -r 300 -gray \
                -singlefile "$SOURCE_PDF" "$OUTPUT_DIRECTORY/ocrad-input/$page_name"
        fi
        ocrad "$ocrad_input" -o "$OUTPUT_DIRECTORY/ocrad/$page_name.txt" \
            2>"$OUTPUT_DIRECTORY/ocrad/$page_name.stderr"
    fi

    case ",$surya_pages," in
        *",$page_number,"*)
            if [ ! -f "$OUTPUT_DIRECTORY/surya/$page_name/results.json" ]; then
                RECOGNITION_BATCH_SIZE=32 PYTORCH_CUDA_ALLOC_CONF=expandable_segments:True \
                    "$surya_python" -c 'from surya.scripts.ocr_text import ocr_text_cli; ocr_text_cli()' \
                    "$render_path" --output_dir "$OUTPUT_DIRECTORY/surya" --disable_math \
                    >"$OUTPUT_DIRECTORY/surya/$page_name.stdout" \
                    2>"$OUTPUT_DIRECTORY/surya/$page_name.stderr"
            fi
            ;;
    esac

    page_number=$((page_number + 1))
done

(
    cd "$OUTPUT_DIRECTORY"
    find render tesseract-psm6 tesseract-psm11 ocrad-input ocrad metadata -type f \
        ! -path 'metadata/artifacts.sha256' -print0 \
        | sort -z \
        | xargs -0 sha256sum
    if [ -d surya ]; then
        find surya -type f -print0 | sort -z | xargs -0 sha256sum
    fi
) >"$OUTPUT_DIRECTORY/metadata/artifacts.sha256"

printf 'OCR extraction complete: %s\n' "$OUTPUT_DIRECTORY"
