# i4002_gates Constraints for Xilinx Spartan-7 xc7s25csga324
#
# WHY: MODE=gate synthesizes TOP=i4002_gates from
#      docs/evidence/verilog_v0/4002/i4002_gates.v. Every port on that
#      module header is constrained here.
# WHAT: Spartan-7 xc7s25csga324 pin assignment for the i4002_gates top.
# HOW:  vivado -source ... ; read_xdc i4002_gates_spartan7.xdc
#
# module i4002_gates (
#     input wire VDD, VSS, CM
# );

# ============================================================
# Rail ports (structural model only; see i4001_gates_spartan7.xdc header)
# ============================================================

set_property -dict {PACKAGE_PIN M13 IOSTANDARD LVCMOS33} [get_ports VDD]
set_property -dict {PACKAGE_PIN M14 IOSTANDARD LVCMOS33} [get_ports VSS]

# ============================================================
# Command line input
# ============================================================

set_property -dict {PACKAGE_PIN N13 IOSTANDARD LVCMOS33} [get_ports CM]
