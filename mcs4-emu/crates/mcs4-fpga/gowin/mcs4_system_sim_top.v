// Deterministic simulation wrapper for the shared MCS-4 system core.

module mcs4_system_sim_top #(
  parameter CLOCKS_PER_BIT = 4,
  parameter PHI_HALF = 2,
  parameter ROM_INIT_FILE = "monitor_rom.hex"
)(
  input  wire sys_clk,
  input  wire rst,
  input  wire test_in,
  input  wire uart_rx,
  output wire uart_tx,
  output wire led_d3,
  output wire debug_phi1,
  output wire debug_phi2,
  output wire [3:0] debug_bus_data,
  output wire debug_cpu_data_oe,
  output wire debug_rom_data_oe,
  output wire debug_ram_data_oe,
  output wire debug_bus_driven,
  output wire debug_cm_rom,
  output wire debug_cm_ram,
  output wire debug_rom_selected,
  output wire debug_ram_selected,
  output wire [11:0] debug_cpu_pc,
  output wire [3:0] debug_cpu_accumulator,
  output wire debug_cpu_carry,
  output wire [2:0] debug_cpu_phase,
  output wire debug_wmp_strobe,
  output wire [3:0] debug_wmp_data,
  output wire debug_rom_io_rd,
  output wire debug_uart_rx_ready
);

  wire phi1;
  wire phi2;

  assign debug_phi1 = phi1;
  assign debug_phi2 = phi2;

  clock_gen #(
    .HALF_PERIOD(PHI_HALF)
  ) u_clock (
    .sys_clk(sys_clk),
    .rst(rst),
    .phi1(phi1),
    .phi2(phi2)
  );

  mcs4_system_core #(
    .CLOCKS_PER_BIT(CLOCKS_PER_BIT),
    .ROM_INIT_FILE(ROM_INIT_FILE)
  ) u_core (
    .sys_clk(sys_clk),
    .rst(rst),
    .phi1(phi1),
    .phi2(phi2),
    .test_in(test_in),
    .uart_rx(uart_rx),
    .uart_tx(uart_tx),
    .led_heartbeat(led_d3),
    .debug_bus_data(debug_bus_data),
    .debug_cpu_data_oe(debug_cpu_data_oe),
    .debug_rom_data_oe(debug_rom_data_oe),
    .debug_ram_data_oe(debug_ram_data_oe),
    .debug_bus_driven(debug_bus_driven),
    .debug_cm_rom(debug_cm_rom),
    .debug_cm_ram(debug_cm_ram),
    .debug_rom_selected(debug_rom_selected),
    .debug_ram_selected(debug_ram_selected),
    .debug_cpu_pc(debug_cpu_pc),
    .debug_cpu_accumulator(debug_cpu_accumulator),
    .debug_cpu_carry(debug_cpu_carry),
    .debug_cpu_phase(debug_cpu_phase),
    .debug_wmp_strobe(debug_wmp_strobe),
    .debug_wmp_data(debug_wmp_data),
    .debug_rom_io_rd(debug_rom_io_rd),
    .debug_uart_rx_ready(debug_uart_rx_ready)
  );

endmodule
