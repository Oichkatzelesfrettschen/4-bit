# Developer Proof Bundle

The developer proof bundle packages reproducible i4003 virtual-board evidence
from one clean Git revision. It is not a release package, a bitstream, a board
programming artifact, or a physical conformance claim.

Run it only from a clean working tree:

~~~sh
just developer-bundle
~~~

The command writes `target/developer-bundle/<full-git-revision>/` and refuses
to overwrite an existing bundle. It retains a deterministic `git archive`, the
generated i4003 FPGA Verilog plus exporter provenance, headless VCD and JSON
scenario outputs, validation logs, SHA-256 checksums, and a bundle manifest.

The builder runs `just verify`, configures a release CMake build of the Qt6 and
Verilator board, runs CTest, then exports the retained HDL again into the bundle.
It compares that HDL against the module used by CTest and rejects a dirty or
revision-mismatched exporter manifest.

The bundle intentionally excludes executables, host libraries, board
constraints, bitstreams, programming commands, credentials, and release
metadata. A release contract requires separate target, synthesis, install,
checksum, rollback, and attended-hardware evidence.
