# MCS-4 Gowin Synthesis Script for GW1N-2 LQFP100
#
# WHY: Automates synthesis + PnR + bitstream for the KIWI FPGA board.
# HOW: QT_QPA_PLATFORM=offscreen gw_sh gowin/mcs4_gowin.tcl
#
# Reference: SUG1220-1.1E Gowin Software Tcl Commands User Guide

set_device -name GW1N-2 GW1N-UV2LQ100C6/I5

add_file gowin/mcs4_system_core.v
add_file gowin/mcs4_top.v
add_file gowin/clock_gen.v
add_file gowin/uart_hw.v
add_file gowin/uart_bridge.v
add_file gowin/rom_bsram.v
add_file gowin/ram_bsram.v
add_file build/i4004_fpga.v
add_file build/i4001_fpga.v
add_file build/i4002_fpga.v
add_file constraints/mcs4_gowin.cst
if {[info exists ::env(MCS4_GOWIN_SDC)] && $::env(MCS4_GOWIN_SDC) ne ""} {
    if {![file exists $::env(MCS4_GOWIN_SDC)]} {
        error "deployment timing-constraints file does not exist: $::env(MCS4_GOWIN_SDC)"
    }
    add_file $::env(MCS4_GOWIN_SDC)
    puts "Using reviewed deployment timing constraints: $::env(MCS4_GOWIN_SDC)"
} else {
    puts "No reviewed deployment timing constraints selected; synthesis remains non-deployable."
}

set_option -top_module mcs4_top
set_option -output_base_name mcs4_system
set_option -use_sspi_as_gpio 1
set_option -use_mspi_as_gpio 1

run all

puts ""
puts "=== MCS-4 synthesis complete ==="
puts "Hardware deployment requires reviewed sys_clk_in route evidence."
