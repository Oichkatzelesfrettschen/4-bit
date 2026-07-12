# Resource estimation: synthesize against GW1N-9C (proxy for GW1N-2)
# GW1N-9C is supported by Education edition; GW1N-2 needs commercial license
# Uses the external-clock mcs4_proxy wrapper.
set_device GW1N-LV9LQ144C6/I5
add_file gowin/mcs4_proxy.v
add_file gowin/mcs4_system_core.v
add_file gowin/clock_gen.v
add_file gowin/uart_hw.v
add_file gowin/uart_bridge.v
add_file gowin/rom_bsram.v
add_file gowin/ram_bsram.v
add_file build/i4004_fpga.v
add_file build/i4001_fpga.v
add_file build/i4002_fpga.v
set_option -top_module mcs4_proxy
set_option -output_base_name mcs4_estimate
run all
