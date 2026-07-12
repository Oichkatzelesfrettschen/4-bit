"""Focused unit tests for the generated HDL validation contract."""

from __future__ import annotations

import verify_hdl_exports as hdl


def test_expected_module_name_tracks_typed_export_flavors() -> None:
    """The checked command spelling maps to the exact emitted top name."""
    assert hdl.expected_module_name("behavioral", "4003") == "i4003"
    assert hdl.expected_module_name("fpga", "4003") == "i4003_fpga"


def test_verilator_warning_allowlist_is_module_scoped() -> None:
    """Only the documented behavioral power-on initializer warning is accepted."""
    allowlist = hdl.load_warning_allowlist()
    output = "%Warning-PROCASSINIT: i4003.v:1:1: initializer\n"
    assert hdl.unexpected_warning_codes("behavioral", "i4003", output, allowlist) == set()
    assert hdl.unexpected_warning_codes("fpga", "i4003_fpga", output, allowlist) == {"PROCASSINIT"}
    assert hdl.unexpected_warning_codes("behavioral", "i4003", "%Warning-WIDTH: x\n", allowlist) == {
        "WIDTH"
    }
    assert hdl.missing_warning_codes("behavioral", "i4003", "", allowlist) == {"PROCASSINIT"}
