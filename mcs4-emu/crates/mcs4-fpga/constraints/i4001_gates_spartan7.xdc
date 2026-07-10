# i4001_gates Constraints for Xilinx Spartan-7 xc7s25csga324
#
# WHY: MODE=gate synthesizes TOP=i4001_gates from
#      docs/evidence/verilog_v0/4001/i4001_gates.v. Every port on that
#      module header is constrained here, and nothing else.
# WHAT: Spartan-7 xc7s25csga324 pin assignment for the i4001_gates top.
# HOW:  vivado -source ... ; read_xdc i4001_gates_spartan7.xdc
#
# module i4001_gates (
#     input  wire VDD, VSS, D0, IO0,
#     output wire CL, CM, D2, D3
# );

# ============================================================
# Rail ports (structural model only; VDD/VSS are logic ports on
# this gate-level netlist, not the fabric's real supply rails --
# tie them to fixed-level pins pending board bring-up)
# ============================================================

set_property -dict {PACKAGE_PIN M13 IOSTANDARD LVCMOS33} [get_ports VDD]
set_property -dict {PACKAGE_PIN M14 IOSTANDARD LVCMOS33} [get_ports VSS]

# ============================================================
# Data I/O
# ============================================================

set_property -dict {PACKAGE_PIN N13 IOSTANDARD LVCMOS33} [get_ports D0]
set_property -dict {PACKAGE_PIN N14 IOSTANDARD LVCMOS33} [get_ports IO0]
set_property -dict {PACKAGE_PIN P13 IOSTANDARD LVCMOS33} [get_ports CL]
set_property -dict {PACKAGE_PIN P14 IOSTANDARD LVCMOS33} [get_ports CM]
set_property -dict {PACKAGE_PIN R13 IOSTANDARD LVCMOS33} [get_ports D2]
set_property -dict {PACKAGE_PIN R14 IOSTANDARD LVCMOS33} [get_ports D3]
