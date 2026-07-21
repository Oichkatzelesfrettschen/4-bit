#!/usr/bin/env sh
# Capture reproducible static and dynamic evidence for the Rust execution paths.
set -eu

REPO_ROOT=$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)
OUTPUT_DIR=${1:-"$REPO_ROOT/target/callgraph-capture"}
CARGO_CAPTURE_TARGET_DIR=${CALLGRAPH_CAPTURE_CARGO_TARGET_DIR:-"$REPO_ROOT/target"}

case "$OUTPUT_DIR" in
    /*) ;;
    *) OUTPUT_DIR="$REPO_ROOT/$OUTPUT_DIR" ;;
esac

case "$CARGO_CAPTURE_TARGET_DIR" in
    /*) ;;
    *) CARGO_CAPTURE_TARGET_DIR="$REPO_ROOT/$CARGO_CAPTURE_TARGET_DIR" ;;
esac

CAPTURE_TMPDIR=${CALLGRAPH_CAPTURE_TMPDIR:-"$OUTPUT_DIR/tmp"}

case "$CAPTURE_TMPDIR" in
    /*) ;;
    *) CAPTURE_TMPDIR="$REPO_ROOT/$CAPTURE_TMPDIR" ;;
esac

if [ -d "$OUTPUT_DIR" ] && [ -n "$(find "$OUTPUT_DIR" -mindepth 1 -maxdepth 1 -print -quit)" ]; then
    printf '%s\n' "capture directory is not empty: $OUTPUT_DIR" >&2
    printf '%s\n' "choose a new directory so prior evidence remains intact" >&2
    exit 2
fi

mkdir -p "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/cflow"
mkdir -p "$OUTPUT_DIR/cscope"
mkdir -p "$OUTPUT_DIR/mir"
mkdir -p "$OUTPUT_DIR/modules"
mkdir -p "$OUTPUT_DIR/python"
mkdir -p "$OUTPUT_DIR/runtime"
mkdir -p "$OUTPUT_DIR/static"
mkdir -p "$OUTPUT_DIR/source"
mkdir -p "$CAPTURE_TMPDIR"

# Keep compiler and profiler temporaries inside the retained capture root.
# Some hosts mount /tmp with a smaller quota than the repository filesystem.
TMPDIR="$CAPTURE_TMPDIR"
export TMPDIR

cd "$REPO_ROOT"

require_command() {
    command_name=$1
    if ! command -v "$command_name" >/dev/null 2>&1; then
        printf '%s\n' "required command not found: $command_name" >&2
        exit 127
    fi
}

for command_name in cflow cscope cargo cmake ninja python3 sha256sum strace tar valgrind \
    verilator callgrind_annotate gprof2dot dot; do
    require_command "$command_name"
done

{
    printf 'repository=%s\n' "$REPO_ROOT"
    printf 'commit=%s\n' "$(git rev-parse HEAD)"
    printf 'branch=%s\n' "$(git branch --show-current)"
    printf 'source_date_epoch=%s\n' "$(git log -1 --format=%ct)"
    printf 'cargo_target_dir=%s\n' "$CARGO_CAPTURE_TARGET_DIR"
    printf 'temporary_directory=%s\n' "$TMPDIR"
    for command_name in cflow cscope cargo cmake ninja rustc python3 strace \
        valgrind verilator callgrind_annotate gprof2dot dot; do
        printf 'tool_%s_path=%s\n' "$command_name" "$(command -v "$command_name")"
        tool_version=$("$command_name" --version 2>&1 | sed -n '1p' || true)
        printf 'tool_%s_version=%s\n' "$command_name" "$tool_version"
    done
    printf '%s\n' 'status:'
    git status --short
} > "$OUTPUT_DIR/environment.txt"

# Include tracked source plus nonignored worktree files. A capture can analyze
# an intentionally dirty implementation, so a tracked-only inventory would
# omit the exact new source that the later targeted probes execute.
git ls-files --cached --others --exclude-standard -- '*.rs' | LC_ALL=C sort \
    > "$OUTPUT_DIR/static/rust.files"
git ls-files --cached --others --exclude-standard -- '*.py' | LC_ALL=C sort \
    > "$OUTPUT_DIR/static/python.files"
git ls-files --cached --others --exclude-standard -- '*.v' | LC_ALL=C sort \
    > "$OUTPUT_DIR/static/verilog.files"
git ls-files --cached --others --exclude-standard -- '*.cpp' '*.cc' '*.cxx' '*.h' '*.hpp' \
    | LC_ALL=C sort > "$OUTPUT_DIR/static/cpp.files"

{
    cat "$OUTPUT_DIR/static/rust.files"
    cat "$OUTPUT_DIR/static/python.files"
    cat "$OUTPUT_DIR/static/verilog.files"
    cat "$OUTPUT_DIR/static/cpp.files"
    printf '%s\n' \
        '.cargo/config.toml' \
        'Cargo.lock' \
        'Cargo.toml' \
        'rust-toolchain.toml' \
        'scripts/callgraph_capture.sh' \
        'scripts/extract_mir_calls.py' \
        'scripts/extract_python_callgraph.py' \
        'scripts/verify_capture_bundle.py' \
        'tools/virtual-fpga/CMakeLists.txt' \
        'tools/virtual-fpga/scenarios/mcs4-system-monitor.json' \
        'tools/virtual-fpga/scenarios/mcs4-system-invalid-budget.json' \
        'tools/virtual-fpga/scenarios/mcs4-common-nop.json' \
        'tools/virtual-fpga/verify_mcs4_system_scenario.cmake' \
        'tools/virtual-fpga/verify_mcs4_system_invalid_scenario.cmake' \
        'tools/virtual-fpga/verify_mcs4_common_stimulus.cmake' \
        'scripts/compare_common_stimulus_traces.py' \
        'scripts/generate_mod40_evidence_contract.py' \
        'scripts/verify_mod40_evidence.py' \
        'docs/evidence/intellec/mod40_component_pin_net_v1.json' \
        'docs/evidence/intellec/mod40_route_ledger_v1.json' \
        'docs/evidence/intellec_sources.yaml' \
        'mcs4-emu/crates/mcs4-fpga/gowin/monitor_rom.hex'
    find mcs4-emu/crates -mindepth 2 -maxdepth 2 -type f -name Cargo.toml | LC_ALL=C sort
} | LC_ALL=C sort -u > "$OUTPUT_DIR/source/inputs.files"

while IFS= read -r source_path; do
    case "$source_path" in
        ''|/*|../*|*/../*)
            printf '%s\n' "invalid capture source path: $source_path" >&2
            exit 1
            ;;
    esac
    if [ ! -f "$source_path" ]; then
        printf '%s\n' "capture source path is not a regular file: $source_path" >&2
        exit 1
    fi
    source_hash=$(sha256sum -- "$source_path" | awk '{print $1}')
    printf '%s\t%s\n' "$source_hash" "$source_path"
done < "$OUTPUT_DIR/source/inputs.files" > "$OUTPUT_DIR/source/inputs.sha256"

tar --create --no-recursion --file="$OUTPUT_DIR/source/inputs.tar" --verbatim-files-from \
    --files-from="$OUTPUT_DIR/source/inputs.files"
git diff --binary HEAD > "$OUTPUT_DIR/source/tracked-working-tree.diff"

if [ "${CALLGRAPH_CAPTURE_RUST_ANALYZER:-0}" = "1" ]; then
    require_command rust-analyzer
    mkdir -p "$OUTPUT_DIR/rust-analyzer"
    if rust-analyzer analysis-stats "$REPO_ROOT" --no-test \
        > "$OUTPUT_DIR/rust-analyzer/analysis-stats.stdout" \
        2> "$OUTPUT_DIR/rust-analyzer/analysis-stats.stderr"; then
        printf 'exit=0\n' > "$OUTPUT_DIR/rust-analyzer/analysis-stats.status"
    else
        exit_code=$?
        printf 'exit=%s\n' "$exit_code" \
            > "$OUTPUT_DIR/rust-analyzer/analysis-stats.status"
    fi
fi

printf '%s\n' 'mcs4-emu/crates/mcs4-system/src/bin/fixture_runner.rs' \
    > "$OUTPUT_DIR/static/fixture-runner.files"
printf '%s\n' 'mcs4-emu/crates/mcs4-gui/src/main.rs' \
    > "$OUTPUT_DIR/static/gui-fixture.files"
printf '%s\n' 'mcs4-emu/crates/mcs4-core/tests/solver_datasheet_timing.rs' \
    > "$OUTPUT_DIR/static/solver-timing.files"
printf '%s\n' 'mcs4-emu/crates/mcs4-fpga/src/verilog.rs' \
    > "$OUTPUT_DIR/static/fpga-export.files"
printf '%s\n' 'mcs4-emu/crates/mcs4-fpga/src/bin/mcs4-fpga-export.rs' \
    > "$OUTPUT_DIR/static/fpga-export-cli.files"
printf '%s\n' 'mcs4-emu/crates/mcs4-chips/src/i4003.rs' \
    > "$OUTPUT_DIR/static/i4003-behavior.files"
printf '%s\n' 'mcs4-emu/crates/mcs4-fpga/src/verilog.rs' \
    > "$OUTPUT_DIR/static/i4003-export.files"
printf '%s\n' 'mcs4-emu/crates/mcs4-system/src/bin/mcs4-phase-trace.rs' \
    > "$OUTPUT_DIR/static/phase-trace.files"
printf '%s\n' \
    'mcs4-emu/crates/mcs4-system/src/bin/mcs4-phase-trace.rs' \
    'mcs4-emu/crates/mcs4-system/src/replay.rs' \
    'mcs4-emu/crates/mcs4-system/src/trace.rs' \
    > "$OUTPUT_DIR/static/trace-replay.files"
printf '%s\n' \
    'mcs4-emu/crates/mcs4-system/src/bin/mcs4-common-stimulus.rs' \
    'mcs4-emu/crates/mcs4-system/src/stimulus.rs' \
    'mcs4-emu/crates/mcs4-system/src/replay.rs' \
    'mcs4-emu/crates/mcs4-system/src/trace.rs' \
    > "$OUTPUT_DIR/static/common-stimulus.files"
printf '%s\n' \
    'mcs4-emu/crates/mcs4-intellec/src/console.rs' \
    'mcs4-emu/crates/mcs4-intellec/src/machine.rs' \
    'mcs4-emu/crates/mcs4-intellec/src/mod40.rs' \
    'mcs4-emu/crates/mcs4-intellec/src/mod40_routes.rs' \
    'mcs4-emu/crates/mcs4-intellec/src/monitor_rom.rs' \
    'mcs4-emu/crates/mcs4-intellec/src/profile.rs' \
    'mcs4-emu/crates/mcs4-intellec/src/replay.rs' \
    'mcs4-emu/crates/mcs4-periph/src/teletype.rs' \
    > "$OUTPUT_DIR/static/intellec-machine.files"
printf '%s\n' \
    'tools/virtual-fpga/system_main.cpp' \
    > "$OUTPUT_DIR/static/virtual-fpga-system.files"
printf '%s\n' 'scripts/gate_to_verilog_v0.py' \
    > "$OUTPUT_DIR/static/gate-export-python.files"
printf '%s\n' 'scripts/build_netlist_v1_v0.py' \
    > "$OUTPUT_DIR/static/netlist-publish-python.files"
printf '%s\n' 'scripts/compare_common_stimulus_traces.py' \
    > "$OUTPUT_DIR/static/common-stimulus-comparison-python.files"
printf '%s\n' \
    'scripts/generate_mod40_evidence_contract.py' \
    'scripts/verify_mod40_evidence.py' \
    > "$OUTPUT_DIR/static/mod40-evidence-python.files"

run_cflow() {
    label=$1
    entrypoint=$2
    input_list=$3

    if xargs cflow "--main=$entrypoint" < "$input_list" \
        > "$OUTPUT_DIR/cflow/$label.txt" \
        2> "$OUTPUT_DIR/cflow/$label.stderr"; then
        printf 'exit=0\n' > "$OUTPUT_DIR/cflow/$label.status"
    else
        exit_code=$?
        printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/cflow/$label.status"
    fi
}

run_cflow fixture-runner main "$OUTPUT_DIR/static/fixture-runner.files"
run_cflow gui-fixture run_fixture "$OUTPUT_DIR/static/gui-fixture.files"
run_cflow solver-datasheet-timing inverter_propagation_delay_fits_datasheet_clock_windows \
    "$OUTPUT_DIR/static/solver-timing.files"
run_cflow fpga-export all_chip_modules "$OUTPUT_DIR/static/fpga-export.files"
run_cflow fpga-export-cli main "$OUTPUT_DIR/static/fpga-export-cli.files"
run_cflow i4003-behavior set_enable_pin "$OUTPUT_DIR/static/i4003-behavior.files"
run_cflow i4003-export chip_i4003 "$OUTPUT_DIR/static/i4003-export.files"
run_cflow phase-trace main "$OUTPUT_DIR/static/phase-trace.files"
run_cflow trace-replay main "$OUTPUT_DIR/static/trace-replay.files"
{
    cat "$OUTPUT_DIR/cflow/trace-replay.status"
    printf '%s\n' 'semantic=0'
    printf '%s\n' 'reason=cflow is lexical-only for Rust source and does not resolve traits, macros, or monomorphization'
} > "$OUTPUT_DIR/cflow/trace-replay.status.tmp"
mv "$OUTPUT_DIR/cflow/trace-replay.status.tmp" "$OUTPUT_DIR/cflow/trace-replay.status"
run_cflow common-stimulus main "$OUTPUT_DIR/static/common-stimulus.files"
{
    cat "$OUTPUT_DIR/cflow/common-stimulus.status"
    printf '%s\n' 'semantic=0'
    printf '%s\n' 'reason=cflow is lexical-only for Rust source and does not resolve traits, macros, or monomorphization'
} > "$OUTPUT_DIR/cflow/common-stimulus.status.tmp"
mv "$OUTPUT_DIR/cflow/common-stimulus.status.tmp" "$OUTPUT_DIR/cflow/common-stimulus.status"
run_cflow intellec-machine step_phase "$OUTPUT_DIR/static/intellec-machine.files"
{
    cat "$OUTPUT_DIR/cflow/intellec-machine.status"
    printf '%s\n' 'semantic=0'
    printf '%s\n' 'reason=cflow is lexical-only for Rust source and does not resolve traits, macros, or monomorphization'
} > "$OUTPUT_DIR/cflow/intellec-machine.status.tmp"
mv "$OUTPUT_DIR/cflow/intellec-machine.status.tmp" "$OUTPUT_DIR/cflow/intellec-machine.status"
run_cflow intellec-mod40-source-gate validate_historical_execution \
    "$OUTPUT_DIR/static/intellec-machine.files"
{
    cat "$OUTPUT_DIR/cflow/intellec-mod40-source-gate.status"
    printf '%s\n' 'semantic=0'
    printf '%s\n' 'reason=cflow is lexical-only for Rust source and does not resolve traits, macros, or monomorphization'
} > "$OUTPUT_DIR/cflow/intellec-mod40-source-gate.status.tmp"
mv "$OUTPUT_DIR/cflow/intellec-mod40-source-gate.status.tmp" \
    "$OUTPUT_DIR/cflow/intellec-mod40-source-gate.status"
run_cflow intellec-mod40-blocked-evidence-gates blocked_evidence_gate_ids \
    "$OUTPUT_DIR/static/intellec-machine.files"
{
    cat "$OUTPUT_DIR/cflow/intellec-mod40-blocked-evidence-gates.status"
    printf '%s\n' 'semantic=0'
    printf '%s\n' 'reason=cflow is lexical-only for Rust source and does not resolve traits, macros, or monomorphization'
} > "$OUTPUT_DIR/cflow/intellec-mod40-blocked-evidence-gates.status.tmp"
mv "$OUTPUT_DIR/cflow/intellec-mod40-blocked-evidence-gates.status.tmp" \
    "$OUTPUT_DIR/cflow/intellec-mod40-blocked-evidence-gates.status"
run_cflow intellec-mod40-evidence-gate-statuses evidence_gate_statuses \
    "$OUTPUT_DIR/static/intellec-machine.files"
{
    cat "$OUTPUT_DIR/cflow/intellec-mod40-evidence-gate-statuses.status"
    printf '%s\n' 'semantic=0'
    printf '%s\n' 'reason=cflow is lexical-only for Rust source and does not resolve traits, macros, or monomorphization'
} > "$OUTPUT_DIR/cflow/intellec-mod40-evidence-gate-statuses.status.tmp"
mv "$OUTPUT_DIR/cflow/intellec-mod40-evidence-gate-statuses.status.tmp" \
    "$OUTPUT_DIR/cflow/intellec-mod40-evidence-gate-statuses.status"
for gate_name in \
    cpu_reset_and_phase_timing_is_traced \
    program_ram_write_timing_is_traced \
    panel_arbitration_is_traced \
    terminal_electrical_timing_is_traced \
    monitor_socket_map_is_traced \
    monitor_data_transform_is_primary_backed \
    accepted_monitor_read_set_count; do
    run_cflow "intellec-mod40-$gate_name" "$gate_name" \
        "$OUTPUT_DIR/static/intellec-machine.files"
    {
        cat "$OUTPUT_DIR/cflow/intellec-mod40-$gate_name.status"
        printf '%s\n' 'semantic=0'
        printf '%s\n' 'reason=cflow is lexical-only for Rust source and does not resolve traits, macros, or monomorphization'
    } > "$OUTPUT_DIR/cflow/intellec-mod40-$gate_name.status.tmp"
    mv "$OUTPUT_DIR/cflow/intellec-mod40-$gate_name.status.tmp" \
        "$OUTPUT_DIR/cflow/intellec-mod40-$gate_name.status"
done
run_cflow virtual-fpga-system main "$OUTPUT_DIR/static/virtual-fpga-system.files"
{
    cat "$OUTPUT_DIR/cflow/virtual-fpga-system.status"
    printf '%s\n' 'semantic=0'
    printf '%s\n' 'reason=cflow is lexical-only for C++ and Verilog source and does not resolve Qt dispatch or HDL execution'
} > "$OUTPUT_DIR/cflow/virtual-fpga-system.status.tmp"
mv "$OUTPUT_DIR/cflow/virtual-fpga-system.status.tmp" "$OUTPUT_DIR/cflow/virtual-fpga-system.status"
run_cflow gate-export-python main "$OUTPUT_DIR/static/gate-export-python.files"
if [ ! -s "$OUTPUT_DIR/cflow/gate-export-python.txt" ] \
    && [ -s "$OUTPUT_DIR/cflow/gate-export-python.stderr" ]; then
    {
        printf 'exit=0\n'
        printf 'usable=0\n'
        printf '%s\n' 'reason=cflow parses C syntax and cannot map Python semantics'
    } > "$OUTPUT_DIR/cflow/gate-export-python.status"
fi
run_cflow netlist-publish-python main "$OUTPUT_DIR/static/netlist-publish-python.files"
if [ ! -s "$OUTPUT_DIR/cflow/netlist-publish-python.txt" ] \
    && [ -s "$OUTPUT_DIR/cflow/netlist-publish-python.stderr" ]; then
    {
        printf 'exit=0\n'
        printf 'usable=0\n'
        printf '%s\n' 'reason=cflow parses C syntax and cannot map Python semantics'
    } > "$OUTPUT_DIR/cflow/netlist-publish-python.status"
fi
run_cflow mod40-evidence-python main "$OUTPUT_DIR/static/mod40-evidence-python.files"
if [ ! -s "$OUTPUT_DIR/cflow/mod40-evidence-python.txt" ] \
    && [ -s "$OUTPUT_DIR/cflow/mod40-evidence-python.stderr" ]; then
    {
        printf 'exit=0\n'
        printf 'usable=0\n'
        printf '%s\n' 'reason=cflow parses C syntax and cannot map Python semantics'
    } > "$OUTPUT_DIR/cflow/mod40-evidence-python.status"
fi

if python3 scripts/extract_python_callgraph.py \
    scripts/gate_to_verilog_v0.py \
    "$OUTPUT_DIR/python/gate_to_verilog_v0-callgraph.txt" \
    > "$OUTPUT_DIR/python/gate_to_verilog_v0-callgraph.stdout" \
    2> "$OUTPUT_DIR/python/gate_to_verilog_v0-callgraph.stderr"; then
    printf 'exit=0\n' > "$OUTPUT_DIR/python/gate_to_verilog_v0-callgraph.status"
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" \
        > "$OUTPUT_DIR/python/gate_to_verilog_v0-callgraph.status"
fi

if python3 scripts/extract_python_callgraph.py \
    scripts/compare_common_stimulus_traces.py \
    "$OUTPUT_DIR/python/common-stimulus-comparison-callgraph.txt" \
    > "$OUTPUT_DIR/python/common-stimulus-comparison-callgraph.stdout" \
    2> "$OUTPUT_DIR/python/common-stimulus-comparison-callgraph.stderr"; then
    printf 'exit=0\n' > "$OUTPUT_DIR/python/common-stimulus-comparison-callgraph.status"
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" \
        > "$OUTPUT_DIR/python/common-stimulus-comparison-callgraph.status"
fi

if python3 scripts/extract_python_callgraph.py \
    scripts/build_netlist_v1_v0.py \
    "$OUTPUT_DIR/python/build_netlist_v1_v0-callgraph.txt" \
    > "$OUTPUT_DIR/python/build_netlist_v1_v0-callgraph.stdout" \
    2> "$OUTPUT_DIR/python/build_netlist_v1_v0-callgraph.stderr"; then
    printf 'exit=0\n' > "$OUTPUT_DIR/python/build_netlist_v1_v0-callgraph.status"
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" \
        > "$OUTPUT_DIR/python/build_netlist_v1_v0-callgraph.status"
fi

if python3 scripts/extract_python_callgraph.py \
    scripts/verify_mod40_evidence.py \
    "$OUTPUT_DIR/python/mod40-evidence-callgraph.txt" \
    > "$OUTPUT_DIR/python/mod40-evidence-callgraph.stdout" \
    2> "$OUTPUT_DIR/python/mod40-evidence-callgraph.stderr"; then
    printf 'exit=0\n' > "$OUTPUT_DIR/python/mod40-evidence-callgraph.status"
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" \
        > "$OUTPUT_DIR/python/mod40-evidence-callgraph.status"
fi

if python3 scripts/extract_python_callgraph.py \
    scripts/generate_mod40_evidence_contract.py \
    "$OUTPUT_DIR/python/mod40-evidence-generation-callgraph.txt" \
    > "$OUTPUT_DIR/python/mod40-evidence-generation-callgraph.stdout" \
    2> "$OUTPUT_DIR/python/mod40-evidence-generation-callgraph.stderr"; then
    printf 'exit=0\n' > "$OUTPUT_DIR/python/mod40-evidence-generation-callgraph.status"
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" \
        > "$OUTPUT_DIR/python/mod40-evidence-generation-callgraph.status"
fi

if cscope -b -q -k -i "$OUTPUT_DIR/static/rust.files" -f "$OUTPUT_DIR/cscope/rust.out" \
    > "$OUTPUT_DIR/cscope/build.stdout" \
    2> "$OUTPUT_DIR/cscope/build.stderr"; then
    printf 'exit=0\n' > "$OUTPUT_DIR/cscope/build.status"
    {
        for symbol in main run_fixture run_cycles step tick solve run all_chip_modules \
            set_enable_pin parallel_outputs_enabled parallel_out serial_out \
            chip_i4003 chip_i4003_fpga apply_input step_phase checkpoint \
            restore_from_checkpoint step_phase_inner from_behavioral_phase validate \
            compare_trace_frames CommonStimulus runPhases IntellecMachine \
            IntellecReplaySession install_monitor_rom apply_event \
            validate_terminal_endpoints apply_terminal_input advance_terminal \
            read_intellec_ram_port Mod40Board Mod40SourceGate \
            validate_historical_execution historical_execution_is_authorized \
            blocked_evidence_gate_ids evidence_gate_statuses MOD40_EVIDENCE_GATE_IDS \
            fidelity_level evidence_snapshot decode_imm628_card_inputs \
            cpu_reset_and_phase_timing_is_traced program_ram_write_timing_is_traced \
            panel_arbitration_is_traced terminal_electrical_timing_is_traced \
            monitor_socket_map_is_traced monitor_data_transform_is_primary_backed \
            accepted_monitor_read_set_count; do
            printf '%s\n' "== definitions: $symbol =="
            cscope -d -f "$OUTPUT_DIR/cscope/rust.out" -L -0 "$symbol" || true
            printf '%s\n' "== lexical callees: $symbol =="
            cscope -d -f "$OUTPUT_DIR/cscope/rust.out" -L -2 "$symbol" || true
        done
    } > "$OUTPUT_DIR/cscope/selected-paths.txt" 2>&1
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/cscope/build.status"
    printf '%s\n' 'cscope index build failed; see build.stderr.' \
        > "$OUTPUT_DIR/cscope/selected-paths.txt"
fi

if cscope -b -q -k -i "$OUTPUT_DIR/static/cpp.files" -f "$OUTPUT_DIR/cscope/cpp.out" \
    > "$OUTPUT_DIR/cscope/cpp-build.stdout" \
    2> "$OUTPUT_DIR/cscope/cpp-build.stderr"; then
    {
        printf 'exit=0\n'
        printf 'semantic=0\n'
        printf '%s\n' 'reason=cscope provides lexical references for C++ source only'
    } > "$OUTPUT_DIR/cscope/cpp.status"
    {
        for symbol in main runHeadless runSystemCycles runPhases traceFrame evaluate \
            commonStimulusRom verifyExpectation writeTraceFrames; do
            printf '%s\n' "== definitions: $symbol =="
            cscope -d -f "$OUTPUT_DIR/cscope/cpp.out" -L -0 "$symbol" || true
            printf '%s\n' "== lexical callees: $symbol =="
            cscope -d -f "$OUTPUT_DIR/cscope/cpp.out" -L -2 "$symbol" || true
        done
    } > "$OUTPUT_DIR/cscope/cpp-selected-paths.txt" 2>&1
else
    {
        printf 'exit=0\n'
        printf 'semantic=0\n'
        printf 'usable=0\n'
        printf '%s\n' 'reason=cscope could not parse the optional C++ source map; see cpp-build.stderr'
    } > "$OUTPUT_DIR/cscope/cpp.status"
    printf '%s\n' 'C++ cscope map unavailable; see cpp-build.stderr.' \
        > "$OUTPUT_DIR/cscope/cpp-selected-paths.txt"
fi

if cscope -b -q -k -i "$OUTPUT_DIR/static/gate-export-python.files" \
    -f "$OUTPUT_DIR/cscope/gate-export-python.out" \
    > "$OUTPUT_DIR/cscope/gate-export-python-build.stdout" \
    2> "$OUTPUT_DIR/cscope/gate-export-python-build.stderr"; then
    {
        printf 'exit=0\n'
        printf 'semantic=0\n'
        printf '%s\n' 'reason=cscope reports lexical tokens only for Python source'
    } > "$OUTPUT_DIR/cscope/gate-export-python.status"
    {
        for symbol in main analyze_gate_export_contract generate_testbench; do
            printf '%s\n' "== definitions: $symbol =="
            cscope -d -f "$OUTPUT_DIR/cscope/gate-export-python.out" -L -0 "$symbol" || true
            printf '%s\n' "== lexical callees: $symbol =="
            cscope -d -f "$OUTPUT_DIR/cscope/gate-export-python.out" -L -2 "$symbol" || true
        done
    } > "$OUTPUT_DIR/cscope/gate-export-python-selected-paths.txt" 2>&1
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" \
        > "$OUTPUT_DIR/cscope/gate-export-python.status"
    printf '%s\n' 'cscope Python lexical index build failed; see gate-export-python-build.stderr.' \
        > "$OUTPUT_DIR/cscope/gate-export-python-selected-paths.txt"
fi

if cscope -b -q -k -i "$OUTPUT_DIR/static/mod40-evidence-python.files" \
    -f "$OUTPUT_DIR/cscope/mod40-evidence-python.out" \
    > "$OUTPUT_DIR/cscope/mod40-evidence-python-build.stdout" \
    2> "$OUTPUT_DIR/cscope/mod40-evidence-python-build.stderr"; then
    {
        printf 'exit=0\n'
        printf 'semantic=0\n'
        printf '%s\n' 'reason=cscope reports lexical tokens only for Python source'
    } > "$OUTPUT_DIR/cscope/mod40-evidence-python.status"
    {
        for symbol in main validate_ledger validate_pin_net_ledger topological_requirement_ids \
            build_status_report write_status_report generate_contract; do
            printf '%s\n' "== definitions: $symbol =="
            cscope -d -f "$OUTPUT_DIR/cscope/mod40-evidence-python.out" -L -0 "$symbol" || true
            printf '%s\n' "== lexical callees: $symbol =="
            cscope -d -f "$OUTPUT_DIR/cscope/mod40-evidence-python.out" -L -2 "$symbol" || true
        done
    } > "$OUTPUT_DIR/cscope/mod40-evidence-python-selected-paths.txt" 2>&1
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/cscope/mod40-evidence-python.status"
    printf '%s\n' 'cscope Python lexical index build failed; see mod40-evidence-python-build.stderr.' \
        > "$OUTPUT_DIR/cscope/mod40-evidence-python-selected-paths.txt"
fi

run_cargo_modules() {
    label=$1
    output_file=$2
    shift 2

    if NO_COLOR=1 cargo modules "$@" \
        > "$OUTPUT_DIR/modules/$output_file" \
        2> "$OUTPUT_DIR/modules/$label.stderr"; then
        printf 'exit=0\n' > "$OUTPUT_DIR/modules/$label.status"
    else
        exit_code=$?
        printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/modules/$label.status"
    fi
}

if cargo modules --version > "$OUTPUT_DIR/modules/cargo-modules.version" \
    2> "$OUTPUT_DIR/modules/cargo-modules-version.stderr"; then
    {
        printf 'exit=0\n'
        printf 'available=1\n'
    } > "$OUTPUT_DIR/modules/cargo-modules-version.status"
    run_cargo_modules system-dependencies mcs4-system-dependencies.dot \
        dependencies --package mcs4-system --lib --no-externs
    run_cargo_modules core-dependencies mcs4-core-dependencies.dot \
        dependencies --package mcs4-core --lib --no-externs
    run_cargo_modules chips-structure mcs4-chips-structure.txt \
        structure --package mcs4-chips --lib
else
    {
        printf 'exit=0\n'
        printf 'available=0\n'
        printf '%s\n' 'reason=cargo-modules is unavailable; MIR remains the compiler-aware graph source'
    } > "$OUTPUT_DIR/modules/cargo-modules-version.status"
    printf '%s\n' 'cargo-modules is unavailable; MIR remains the compiler-aware graph source.' \
        > "$OUTPUT_DIR/modules/cargo-modules.version"
fi

run_mir_binary() {
    label=$1
    package_name=$2
    binary_name=$3
    mir_file="$OUTPUT_DIR/mir/$label.mir"

    if RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR="$CARGO_CAPTURE_TARGET_DIR" CARGO_TERM_COLOR=never \
        cargo rustc --locked -p "$package_name" --bin "$binary_name" -- -Zunpretty=mir \
        > "$mir_file" \
        2> "$OUTPUT_DIR/mir/$label.stderr"; then
        if python3 scripts/extract_mir_calls.py "$mir_file" "$OUTPUT_DIR/mir/$label-calls.tsv" \
            > "$OUTPUT_DIR/mir/$label-extract.stdout" \
            2> "$OUTPUT_DIR/mir/$label-extract.stderr"; then
            printf 'exit=0\n' > "$OUTPUT_DIR/mir/$label-extract.status"
            printf 'exit=0\n' > "$OUTPUT_DIR/mir/$label.status"
        else
            exit_code=$?
            printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/mir/$label-extract.status"
            printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/mir/$label.status"
        fi
    else
        exit_code=$?
        printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/mir/$label.status"
    fi
}

run_mir_library() {
    label=$1
    package_name=$2
    mir_file="$OUTPUT_DIR/mir/$label.mir"

    if RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR="$CARGO_CAPTURE_TARGET_DIR" CARGO_TERM_COLOR=never \
        cargo rustc --locked -p "$package_name" --lib -- -Zunpretty=mir \
        > "$mir_file" \
        2> "$OUTPUT_DIR/mir/$label.stderr"; then
        if python3 scripts/extract_mir_calls.py "$mir_file" "$OUTPUT_DIR/mir/$label-calls.tsv" \
            > "$OUTPUT_DIR/mir/$label-extract.stdout" \
            2> "$OUTPUT_DIR/mir/$label-extract.stderr"; then
            printf 'exit=0\n' > "$OUTPUT_DIR/mir/$label-extract.status"
            printf 'exit=0\n' > "$OUTPUT_DIR/mir/$label.status"
        else
            exit_code=$?
            printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/mir/$label-extract.status"
            printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/mir/$label.status"
        fi
    else
        exit_code=$?
        printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/mir/$label.status"
    fi
}

run_mir_binary fixture-runner mcs4-system fixture_runner
run_mir_binary phase-trace mcs4-system mcs4-phase-trace
run_mir_binary common-stimulus mcs4-system mcs4-common-stimulus
run_mir_binary gui-fixture mcs4-gui mcs4-emu
run_mir_binary fpga-export-cli mcs4-fpga mcs4-fpga-export
run_mir_library system-library mcs4-system
run_mir_library chips-library mcs4-chips
run_mir_library core-library mcs4-core
run_mir_library fpga-library mcs4-fpga
run_mir_library intellec-library mcs4-intellec

if grep -Fqx 'exit=0' "$OUTPUT_DIR/mir/chips-library.status"; then
    {
        sed -n '1p' "$OUTPUT_DIR/mir/chips-library-calls.tsv"
        grep -E 'i4003::|I4003::' "$OUTPUT_DIR/mir/chips-library-calls.tsv" || true
    } > "$OUTPUT_DIR/mir/i4003-behavior-calls.tsv"
    printf 'exit=0\n' > "$OUTPUT_DIR/mir/i4003-behavior-calls.status"
else
    printf 'exit=1\n' > "$OUTPUT_DIR/mir/i4003-behavior-calls.status"
fi

if grep -Fqx 'exit=0' "$OUTPUT_DIR/mir/fpga-library.status"; then
    {
        sed -n '1p' "$OUTPUT_DIR/mir/fpga-library-calls.tsv"
        awk -F '\t' '$1 == "chip_i4003" || $1 == "chip_i4003_fpga" || \
            $1 == "all_chip_modules" || $1 == "fpga_chip_modules" { print }' \
            "$OUTPUT_DIR/mir/fpga-library-calls.tsv"
    } > "$OUTPUT_DIR/mir/i4003-export-calls.tsv"
    printf 'exit=0\n' > "$OUTPUT_DIR/mir/i4003-export-calls.status"
else
    printf 'exit=1\n' > "$OUTPUT_DIR/mir/i4003-export-calls.status"
fi

if grep -Fqx 'exit=0' "$OUTPUT_DIR/mir/phase-trace.status" \
    && grep -Fqx 'exit=0' "$OUTPUT_DIR/mir/system-library.status"; then
    {
        printf '%s\n' 'source\tcaller\tcallee\tmir_line'
        awk 'NR > 1 && /ReplaySession|ReplayInput|ReplayCheckpoint|TraceFrame|step_phase|apply_input|checkpoint|restore_from_checkpoint|from_behavioral_phase|compare_trace_frames/ { print "phase-trace\t" $0 }' \
            "$OUTPUT_DIR/mir/phase-trace-calls.tsv"
        awk 'NR > 1 && /ReplaySession|ReplayInput|ReplayCheckpoint|TraceFrame|step_phase|apply_input|checkpoint|restore_from_checkpoint|from_behavioral_phase|compare_trace_frames/ { print "system-library\t" $0 }' \
            "$OUTPUT_DIR/mir/system-library-calls.tsv"
    } > "$OUTPUT_DIR/mir/trace-replay-calls.tsv"
    printf 'exit=0\n' > "$OUTPUT_DIR/mir/trace-replay-calls.status"
else
    printf 'exit=1\n' > "$OUTPUT_DIR/mir/trace-replay-calls.status"
fi

if grep -Fqx 'exit=0' "$OUTPUT_DIR/mir/intellec-library.status"; then
    {
        sed -n '1p' "$OUTPUT_DIR/mir/intellec-library-calls.tsv"
        awk 'NR > 1 && /IntellecMachine|IntellecReplaySession|IntellecPanel|Teletype33|MonitorRom|Mod40Board|Mod40SourceGate|monitor_select_decode_outputs_are_recorded|step_phase|apply_event|install_monitor_rom/ { print }' \
            "$OUTPUT_DIR/mir/intellec-library-calls.tsv"
    } > "$OUTPUT_DIR/mir/intellec-machine-calls.tsv"
    printf 'exit=0\n' > "$OUTPUT_DIR/mir/intellec-machine-calls.status"
else
    printf 'exit=1\n' > "$OUTPUT_DIR/mir/intellec-machine-calls.status"
fi

if CARGO_TARGET_DIR="$CARGO_CAPTURE_TARGET_DIR" CARGO_TERM_COLOR=never \
    cargo build --locked -p mcs4-gui \
    > "$OUTPUT_DIR/runtime/build.stdout" \
    2> "$OUTPUT_DIR/runtime/build.stderr" \
    && CARGO_TARGET_DIR="$CARGO_CAPTURE_TARGET_DIR" CARGO_TERM_COLOR=never \
    cargo build --locked -p mcs4-system --bin fixture_runner --bin mcs4-phase-trace \
        --bin mcs4-common-stimulus \
    >> "$OUTPUT_DIR/runtime/build.stdout" \
    2>> "$OUTPUT_DIR/runtime/build.stderr" \
    && CARGO_TARGET_DIR="$CARGO_CAPTURE_TARGET_DIR" CARGO_TERM_COLOR=never \
    cargo build --locked -p mcs4-fpga --bin mcs4-fpga-export \
    >> "$OUTPUT_DIR/runtime/build.stdout" \
    2>> "$OUTPUT_DIR/runtime/build.stderr" \
    && CARGO_TARGET_DIR="$CARGO_CAPTURE_TARGET_DIR" CARGO_TERM_COLOR=never \
    cargo build --locked -p mcs4-intellec \
    >> "$OUTPUT_DIR/runtime/build.stdout" \
    2>> "$OUTPUT_DIR/runtime/build.stderr"; then
    printf 'exit=0\n' > "$OUTPUT_DIR/runtime/build.status"
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/runtime/build.status"
fi

GUI_BINARY="$CARGO_CAPTURE_TARGET_DIR/debug/mcs4-emu"
FIXTURE_BINARY="$CARGO_CAPTURE_TARGET_DIR/debug/fixture_runner"
PHASE_TRACE_BINARY="$CARGO_CAPTURE_TARGET_DIR/debug/mcs4-phase-trace"
COMMON_STIMULUS_BINARY="$CARGO_CAPTURE_TARGET_DIR/debug/mcs4-common-stimulus"
FIXTURE_PATH="$REPO_ROOT/mcs4-emu/crates/mcs4-system/fixtures/src_wrm_rdm.hex"
VIRTUAL_FPGA_BUILD_DIRECTORY="$OUTPUT_DIR/cpp-build"
VIRTUAL_SYSTEM_BINARY="$VIRTUAL_FPGA_BUILD_DIRECTORY/mcs4-virtual-system"
VIRTUAL_SYSTEM_SCENARIO="$REPO_ROOT/tools/virtual-fpga/scenarios/mcs4-system-monitor.json"
COMMON_STIMULUS_SCENARIO="$REPO_ROOT/tools/virtual-fpga/scenarios/mcs4-common-nop.json"

if cmake -S "$REPO_ROOT/tools/virtual-fpga" -B "$VIRTUAL_FPGA_BUILD_DIRECTORY" \
    -G Ninja -DBUILD_TESTING=ON \
    > "$OUTPUT_DIR/runtime/virtual-fpga-build.stdout" \
    2> "$OUTPUT_DIR/runtime/virtual-fpga-build.stderr" \
    && cmake --build "$VIRTUAL_FPGA_BUILD_DIRECTORY" \
    >> "$OUTPUT_DIR/runtime/virtual-fpga-build.stdout" \
    2>> "$OUTPUT_DIR/runtime/virtual-fpga-build.stderr"; then
    printf 'exit=0\n' > "$OUTPUT_DIR/runtime/virtual-fpga-build.status"
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/runtime/virtual-fpga-build.status"
fi

run_strace() {
    label=$1
    shift
    if CARGO_TARGET_DIR="$CARGO_CAPTURE_TARGET_DIR" \
        RUST_LOG=mcs4_system=trace,mcs4_chips::i4004=trace \
        strace -ff -ttt -s 256 -o "$OUTPUT_DIR/runtime/$label.strace" "$@" \
        > "$OUTPUT_DIR/runtime/$label.stdout" \
        2> "$OUTPUT_DIR/runtime/$label.stderr"; then
        printf 'exit=0\n' > "$OUTPUT_DIR/runtime/$label.status"
    else
        exit_code=$?
        printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/runtime/$label.status"
    fi
}

run_strace_in_dir() {
    label=$1
    working_directory=$2
    shift 2
    if (
        cd "$working_directory"
        CARGO_TARGET_DIR="$CARGO_CAPTURE_TARGET_DIR" \
            RUST_LOG=mcs4_system=trace,mcs4_chips::i4004=trace \
            strace -ff -ttt -s 256 -o "$OUTPUT_DIR/runtime/$label.strace" "$@" \
            > "$OUTPUT_DIR/runtime/$label.stdout" \
            2> "$OUTPUT_DIR/runtime/$label.stderr"
    ); then
        printf 'exit=0\n' > "$OUTPUT_DIR/runtime/$label.status"
    else
        exit_code=$?
        printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/runtime/$label.status"
    fi
}

run_strace mcs4-fixture "$GUI_BINARY" --mode fixture --system mcs4 \
    --fixture src_wrm_rdm --cycles 12 --strict-io-phases
run_strace mcs40-fixture "$GUI_BINARY" --mode fixture --system mcs40 \
    --fixture src_wrm_rdm --cycles 12 --strict-io-phases
run_strace fixture-runner "$FIXTURE_BINARY" "$FIXTURE_PATH" 12
run_strace phase-trace "$PHASE_TRACE_BINARY" --architecture mcs4 \
    --fixture "$FIXTURE_PATH" --warmup 32 --phases 24
run_strace trace-frame-capture "$PHASE_TRACE_BINARY" --architecture mcs4 \
    --fixture "$FIXTURE_PATH" --warmup 32 --phases 24 --format frame-jsonl \
    --checkpoint "$OUTPUT_DIR/runtime/mcs4-trace-replay.checkpoint.json"
run_strace common-stimulus "$COMMON_STIMULUS_BINARY" --stimulus "$COMMON_STIMULUS_SCENARIO"
run_strace fpga-export-cli "$CARGO_CAPTURE_TARGET_DIR/debug/mcs4-fpga-export" \
    --chip i4003 --flavor fpga \
    --output "$OUTPUT_DIR/runtime/i4003_fpga.v" \
    --manifest "$OUTPUT_DIR/runtime/i4003_fpga.v.manifest.json"
run_strace netlist-v1-build python3 scripts/build_netlist_v1_v0.py \
    --chip 4003 --out-dir "$OUTPUT_DIR/runtime/netlist-v1"
run_strace solver-datasheet-test cargo test --locked -p mcs4-core \
    --test solver_datasheet_timing -- --nocapture
run_strace mcs40-integration-test cargo test --locked -p mcs4-system \
    --test mcs40_4308_integration -- --nocapture
run_strace fpga-export-test cargo test --locked -p mcs4-fpga --lib i4004_fpga -- --nocapture
run_strace i4003-behavior-test cargo test --locked -p mcs4-chips \
    i4003::tests::enable_high_masks_parallel_outputs_without_stopping_shift -- --nocapture
run_strace i4003-system-wiring-test cargo test --locked -p mcs4-system \
    test_i4003_shifts_from_ram_output_port -- --nocapture
run_strace i4003-fpga-export-test cargo test --locked -p mcs4-fpga i4003_ -- --nocapture
run_strace trace-replay-cli-test cargo test --locked -p mcs4-system \
    --test phase_trace_cli frame_jsonl_capture_has_provenance_and_a_restorable_checkpoint -- --nocapture
run_strace trace-frame-comparison-test cargo test --locked -p mcs4-system \
    --test cross_fidelity_trace_fixture -- --nocapture
run_strace intellec-source-gate-test cargo test --locked -p mcs4-intellec \
    replay::tests::historical_profile_rejects_phase_advance_without_evidence -- --nocapture
run_strace intellec-replay-test cargo test --locked -p mcs4-intellec \
    replay::tests::checkpoint_replays_explicit_panel_and_terminal_events -- --nocapture
run_strace mod40-fidelity-test cargo test --locked -p mcs4-intellec \
    mod40::tests::new_board_reports_documented_inventory_fidelity -- --nocapture
run_strace mod40-card-input-test cargo test --locked -p mcs4-intellec \
    mod40_routes::tests::card_input_decoder_preserves_verified_active_low_boundaries -- --nocapture
run_strace mod40-gui-dashboard-test cargo test --locked -p mcs4-gui \
    panels::intellec::tests::mod40_dashboard_owns_a_non_executable_board -- --nocapture
run_strace mod40-evidence-validator python3 scripts/verify_mod40_evidence.py
run_strace mod40-evidence-generation-check python3 scripts/generate_mod40_evidence_contract.py --check
run_strace_in_dir virtual-fpga-system "$VIRTUAL_FPGA_BUILD_DIRECTORY" \
    "$VIRTUAL_SYSTEM_BINARY" --headless --scenario "$VIRTUAL_SYSTEM_SCENARIO" \
    --vcd "$OUTPUT_DIR/runtime/mcs4-system-monitor.vcd" \
    --summary "$OUTPUT_DIR/runtime/mcs4-system-monitor.summary.json" \
    --trace-frames "$OUTPUT_DIR/runtime/mcs4-system-monitor.trace.jsonl"
run_strace_in_dir virtual-fpga-common-stimulus "$VIRTUAL_FPGA_BUILD_DIRECTORY" \
    "$VIRTUAL_SYSTEM_BINARY" --headless --scenario "$COMMON_STIMULUS_SCENARIO" \
    --vcd "$OUTPUT_DIR/runtime/mcs4-common.vcd" \
    --summary "$OUTPUT_DIR/runtime/mcs4-common.summary.json" \
    --trace-frames "$OUTPUT_DIR/runtime/mcs4-common-fpga.trace.jsonl"

if valgrind --vgdb=no --tool=callgrind \
    --callgrind-out-file="$OUTPUT_DIR/runtime/mcs4-fixture.callgrind" \
    "$GUI_BINARY" --mode fixture --system mcs4 --fixture src_wrm_rdm --cycles 64 \
    --strict-io-phases \
    > "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind.stdout" \
    2> "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind.stderr"; then
    if callgrind_annotate --auto=yes "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind" \
        > "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind.txt" \
        2> "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind-annotate.stderr"; then
        printf 'exit=0\n' > "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind-annotate.status"
    else
        exit_code=$?
        printf 'exit=%s\n' "$exit_code" \
            > "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind-annotate.status"
    fi
    if gprof2dot -f callgrind "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind" \
        -o "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind.dot" \
        > "$OUTPUT_DIR/runtime/mcs4-fixture.gprof2dot.stdout" \
        2> "$OUTPUT_DIR/runtime/mcs4-fixture.gprof2dot.stderr" \
        && dot -Tsvg "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind.dot" \
            -o "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind.svg"; then
        printf 'exit=0\n' > "$OUTPUT_DIR/runtime/mcs4-fixture.gprof2dot.status"
    else
        exit_code=$?
        printf 'exit=%s\n' "$exit_code" \
            > "$OUTPUT_DIR/runtime/mcs4-fixture.gprof2dot.status"
    fi
    printf 'exit=0\n' > "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind.status"
else
    exit_code=$?
    printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind.status"
    printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/runtime/mcs4-fixture.callgrind-annotate.status"
    printf 'exit=%s\n' "$exit_code" > "$OUTPUT_DIR/runtime/mcs4-fixture.gprof2dot.status"
fi

python3 scripts/verify_capture_bundle.py --capture-dir "$OUTPUT_DIR" --write
python3 scripts/verify_capture_bundle.py --capture-dir "$OUTPUT_DIR" --require-success

printf '%s\n' "Capture complete: $OUTPUT_DIR"
