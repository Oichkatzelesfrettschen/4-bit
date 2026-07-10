# i4003_gates Constraints for Xilinx Spartan-7 xc7s25csga324
#
# WHY: MODE=gate synthesizes TOP=i4003_gates from
#      docs/evidence/verilog_v0/4003/i4003_gates.v. Every port on that
#      module header is constrained here.
# WHAT: Spartan-7 xc7s25csga324 pin assignment for the i4003_gates top.
# HOW:  vivado -source ... ; read_xdc i4003_gates_spartan7.xdc
#
# module i4003_gates (
#     input  wire VDD, VSS, Q2, Q5, Q6,
#     output wire Q4
# );
#
# NOTE: the extracted netlist names these ports Q2/Q5/Q6/Q4 (net-derived
# from the transistor extraction pass), not CLOCK/DATA/EN as the 4003
# datasheet pinout would suggest. No port carries a CLOCK/CLK name, so no
# create_clock constraint is applied; treat all three inputs as
# asynchronous until the extraction is cross-checked against the
# datasheet pin function to identify the true clock net.

# ============================================================
# Rail ports (structural model only; see i4001_gates_spartan7.xdc header)
# ============================================================

set_property -dict {PACKAGE_PIN M13 IOSTANDARD LVCMOS33} [get_ports VDD]
set_property -dict {PACKAGE_PIN M14 IOSTANDARD LVCMOS33} [get_ports VSS]

# ============================================================
# Shift register inputs / cascade output
# ============================================================

set_property -dict {PACKAGE_PIN N13 IOSTANDARD LVCMOS33} [get_ports Q2]
set_property -dict {PACKAGE_PIN N14 IOSTANDARD LVCMOS33} [get_ports Q5]
set_property -dict {PACKAGE_PIN P13 IOSTANDARD LVCMOS33} [get_ports Q6]
set_property -dict {PACKAGE_PIN P14 IOSTANDARD LVCMOS33} [get_ports Q4]
