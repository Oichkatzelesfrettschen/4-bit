# MCS-4 System Constraints for Xilinx Spartan-7 XC7S15-FTGB196
#
# WHY: Maps MCS-4 signals to Spartan-7 pins for FPGA deployment.
# WHAT: Full MCS-4/MCS-40 system on Spartan-7 (12,800 LUTs available).
# HOW: Use with Vivado: read_xdc mcs4_spartan7.xdc

# ============================================================
# Clock (2 MHz MCS-4 bus clock, 500ns period)
# ============================================================

create_clock -period 500.0 -name phi1 [get_ports phi1]
set_property -dict {PACKAGE_PIN H16 IOSTANDARD LVCMOS33} [get_ports phi1]
set_property -dict {PACKAGE_PIN G16 IOSTANDARD LVCMOS33} [get_ports phi2]

# ============================================================
# Reset
# ============================================================

set_property -dict {PACKAGE_PIN K16 IOSTANDARD LVCMOS33} [get_ports rst]

# ============================================================
# 4-bit Multiplexed Data Bus
# ============================================================

set_property -dict {PACKAGE_PIN J14 IOSTANDARD LVCMOS33} [get_ports {data[0]}]
set_property -dict {PACKAGE_PIN J15 IOSTANDARD LVCMOS33} [get_ports {data[1]}]
set_property -dict {PACKAGE_PIN K14 IOSTANDARD LVCMOS33} [get_ports {data[2]}]
set_property -dict {PACKAGE_PIN K15 IOSTANDARD LVCMOS33} [get_ports {data[3]}]

# ============================================================
# CPU Control Signals
# ============================================================

set_property -dict {PACKAGE_PIN L14 IOSTANDARD LVCMOS33} [get_ports sync]
set_property -dict {PACKAGE_PIN L15 IOSTANDARD LVCMOS33} [get_ports cm_rom]
set_property -dict {PACKAGE_PIN M14 IOSTANDARD LVCMOS33} [get_ports cm_ram]
set_property -dict {PACKAGE_PIN M15 IOSTANDARD LVCMOS33} [get_ports test_pin]

# ============================================================
# ROM I/O Port
# ============================================================

set_property -dict {PACKAGE_PIN N14 IOSTANDARD LVCMOS33} [get_ports {rom_io[0]}]
set_property -dict {PACKAGE_PIN N15 IOSTANDARD LVCMOS33} [get_ports {rom_io[1]}]
set_property -dict {PACKAGE_PIN P14 IOSTANDARD LVCMOS33} [get_ports {rom_io[2]}]
set_property -dict {PACKAGE_PIN P15 IOSTANDARD LVCMOS33} [get_ports {rom_io[3]}]

# ============================================================
# RAM Output Port
# ============================================================

set_property -dict {PACKAGE_PIN A14 IOSTANDARD LVCMOS33} [get_ports {ram_out[0]}]
set_property -dict {PACKAGE_PIN A15 IOSTANDARD LVCMOS33} [get_ports {ram_out[1]}]
set_property -dict {PACKAGE_PIN B14 IOSTANDARD LVCMOS33} [get_ports {ram_out[2]}]
set_property -dict {PACKAGE_PIN B15 IOSTANDARD LVCMOS33} [get_ports {ram_out[3]}]

# ============================================================
# Timing Constraints
# ============================================================

# Input setup/hold relative to phi1
set_input_delay -clock phi1 10.0 [get_ports {data[*] rst test_pin cm_rom cm_ram}]

# Output delay
set_output_delay -clock phi1 10.0 [get_ports {sync rom_io[*] ram_out[*]}]

# Bus turnaround: data bus changes direction between phases
# Allow 50ns for bus turnaround (10% of 500ns period)
set_multicycle_path 1 -setup -from [get_ports {data[*]}] -to [get_ports {data[*]}]
