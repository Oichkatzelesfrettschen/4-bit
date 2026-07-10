# i4004_gates Constraints for Xilinx Spartan-7 xc7s25csga324
#
# WHY: MODE=gate synthesizes TOP=i4004_gates from
#      docs/evidence/verilog_v0/4004/i4004_gates.v. Every port on that
#      module header is constrained here.
# WHAT: Spartan-7 xc7s25csga324 pin assignment for the i4004_gates top.
# HOW:  vivado -source ... ; read_xdc i4004_gates_spartan7.xdc
#
# module i4004_gates (
#     input  wire VDD, VSS,
#     output wire CLK1, CMRAM0, D0_PAD
# );
#
# NOTE: CLK1 is a module OUTPUT (the extracted 4004 generates its own
# two-phase clock internally); there is no external clock input port to
# constrain with create_clock.

# ============================================================
# Rail ports (structural model only; see i4001_gates_spartan7.xdc header)
# ============================================================

set_property -dict {PACKAGE_PIN M13 IOSTANDARD LVCMOS33} [get_ports VDD]
set_property -dict {PACKAGE_PIN M14 IOSTANDARD LVCMOS33} [get_ports VSS]

# ============================================================
# CPU status/clock outputs
# ============================================================

set_property -dict {PACKAGE_PIN N13 IOSTANDARD LVCMOS33} [get_ports CLK1]
set_property -dict {PACKAGE_PIN N14 IOSTANDARD LVCMOS33} [get_ports CMRAM0]
set_property -dict {PACKAGE_PIN P13 IOSTANDARD LVCMOS33} [get_ports D0_PAD]
