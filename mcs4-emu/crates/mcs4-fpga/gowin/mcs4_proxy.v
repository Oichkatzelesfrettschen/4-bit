// External-clock wrapper for resource estimation.

module mcs4_proxy #(
  parameter CLOCKS_PER_BIT = 234,
  parameter PHI_HALF = 9,
  parameter ROM_INIT_FILE = "monitor_rom.hex"
)(
  input  wire clk_in,
  output wire uart_tx,
  input  wire uart_rx,
  output wire led_d3,
  input  wire btn_s1,
  input  wire btn_s2
);

  reg [2:0] reset_sync;
  reg [1:0] test_sync;
  wire rst = reset_sync[2];
  wire test_in = test_sync[1];
  wire phi1;
  wire phi2;
  wire [3:0] unused_debug_bus_data;
  wire unused_debug_cpu_data_oe;
  wire unused_debug_rom_data_oe;
  wire unused_debug_ram_data_oe;
  wire unused_debug_bus_driven;
  wire unused_debug_cm_rom;
  wire unused_debug_cm_ram;
  wire unused_debug_rom_selected;
  wire unused_debug_ram_selected;
  wire [11:0] unused_debug_cpu_pc;
  wire [3:0] unused_debug_cpu_accumulator;
  wire unused_debug_cpu_carry;
  wire [2:0] unused_debug_cpu_phase;
  wire unused_debug_wmp_strobe;
  wire [3:0] unused_debug_wmp_data;
  wire unused_debug_rom_io_rd;
  wire unused_debug_uart_rx_ready;

  always @(posedge clk_in) begin
    reset_sync <= {reset_sync[1:0], ~btn_s1};
    test_sync <= {test_sync[0], ~btn_s2};
  end

  clock_gen #(
    .HALF_PERIOD(PHI_HALF)
  ) u_clock (
    .sys_clk(clk_in),
    .rst(rst),
    .phi1(phi1),
    .phi2(phi2)
  );

  mcs4_system_core #(
    .CLOCKS_PER_BIT(CLOCKS_PER_BIT),
    .ROM_INIT_FILE(ROM_INIT_FILE)
  ) u_core (
    .sys_clk(clk_in),
    .rst(rst),
    .phi1(phi1),
    .phi2(phi2),
    .test_in(test_in),
    .uart_rx(uart_rx),
    .uart_tx(uart_tx),
    .led_heartbeat(led_d3),
    .debug_bus_data(unused_debug_bus_data),
    .debug_cpu_data_oe(unused_debug_cpu_data_oe),
    .debug_rom_data_oe(unused_debug_rom_data_oe),
    .debug_ram_data_oe(unused_debug_ram_data_oe),
    .debug_bus_driven(unused_debug_bus_driven),
    .debug_cm_rom(unused_debug_cm_rom),
    .debug_cm_ram(unused_debug_cm_ram),
    .debug_rom_selected(unused_debug_rom_selected),
    .debug_ram_selected(unused_debug_ram_selected),
    .debug_cpu_pc(unused_debug_cpu_pc),
    .debug_cpu_accumulator(unused_debug_cpu_accumulator),
    .debug_cpu_carry(unused_debug_cpu_carry),
    .debug_cpu_phase(unused_debug_cpu_phase),
    .debug_wmp_strobe(unused_debug_wmp_strobe),
    .debug_wmp_data(unused_debug_wmp_data),
    .debug_rom_io_rd(unused_debug_rom_io_rd),
    .debug_uart_rx_ready(unused_debug_uart_rx_ready)
  );

endmodule
