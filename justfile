# justfile -- common recipes for the MCS-4/MCS-40 emulator workspace
# WHY: Provides single-command shortcuts for lint, test, format, and docs.
# WHAT: Recipes wrap cargo commands with consistent flags for reproducibility.
# HOW: Run `just <recipe>` from the repo root. Requires `just` (https://github.com/casey/just).

# Check types without building
check:
    cargo check --workspace --locked

# Run all tests
test:
    cargo test --workspace --locked

# Run clippy with warnings as errors (same surface as the CI clippy-all alias)
lint:
    cargo clippy --workspace --locked --all-targets --all-features -- -D warnings

# Check formatting (no writes)
fmt:
    cargo fmt --all -- --check

# Fix formatting (in place)
fmt-fix:
    cargo fmt --all

# Build documentation
doc:
    cargo doc --workspace --no-deps --all-features --locked

# Run Python evidence-pipeline tests
python-test:
    python3 -m pytest scripts/tests

# Lint repository automation and evidence-pipeline scripts
scripts-lint:
    ruff check --no-cache scripts/
    shellcheck -S warning scripts/*.sh

# Verify tracked local Markdown links without network access.
link-check:
    scripts/link_check.sh

# Validate every typed behavioral and FPGA HDL export with the pinned local tools.
hdl-validate:
    python3 scripts/verify_hdl_exports.py

# Require a structurally resolved gate-level HDL export before it is delivered.
gate-contract:
    python3 scripts/gate_to_verilog_v0.py --chips 4003 --check-export-contract --check-generated

# Verify generated netlist hashes and every source-input hash in the canonical v1 manifest.
netlist-validate:
    python3 scripts/verify_netlist_manifest.py

# Verify timing bounds, source locators, and code use sites.
timing-validate:
    python3 scripts/verify_timing_parameters.py

# Verify the capability and ownership matrix against retained evidence paths.
capability-validate:
    python3 scripts/verify_capability_registry.py

# Require a current, registered, and unexpired RustSec advisory exception set.
security-validate:
    cargo deny check advisories --config deny.toml
    python3 scripts/verify_advisory_exceptions.py

# Run full verification: Rust, Python, scripts, gate HDL, and docs.
verify: fmt lint test python-test scripts-lint link-check hdl-validate gate-contract netlist-validate timing-validate capability-validate security-validate doc
    @echo "Repository verification passed."

# Capture cflow, cscope, compiler MIR, syscall, and callgrind evidence.
capture-callgraphs:
    scripts/callgraph_capture.sh

# Configure and build the optional Qt6 plus Verilator virtual FPGA board.
virtual-fpga-build:
    cmake -S tools/virtual-fpga -B build/virtual-fpga -G Ninja
    cmake --build build/virtual-fpga

# Run headless virtual-board scenarios with VCD and JSON-oracle verification.
virtual-fpga-test: virtual-fpga-build
    ctest --test-dir build/virtual-fpga --output-on-failure

# Launch the optional Qt6 virtual FPGA board.
virtual-fpga-run: virtual-fpga-build
    build/virtual-fpga/mcs4-virtual-fpga

# Compile and test the optional board with release compiler settings.
virtual-fpga-release-check:
    cmake -S tools/virtual-fpga -B build/virtual-fpga-release -G Ninja -DCMAKE_BUILD_TYPE=Release
    cmake --build build/virtual-fpga-release
    ctest --test-dir build/virtual-fpga-release --output-on-failure

# Build a clean-revision developer proof bundle for the virtual i4003 board.
developer-bundle:
    python3 scripts/build_developer_bundle.py

full: verify
    @echo "All checks passed."
