`timescale 1ns / 1ps

// Deterministic integration test for the shared MCS-4 system datapath.

module tb_mcs4_system;

  localparam SIM_CLOCKS_PER_BIT = 4;
  localparam SIM_PHI_HALF = 2;
  localparam MAX_SYSTEM_CLOCKS = 5000;

  reg sys_clk;
  reg rst;
  reg test_in;
  reg uart_rx;
  wire uart_tx;
  wire led_d3;
  wire debug_phi1;
  wire debug_phi2;
  wire [3:0] debug_bus_data;
  wire debug_cpu_data_oe;
  wire debug_rom_data_oe;
  wire debug_ram_data_oe;
  wire debug_cm_rom;
  wire debug_cm_ram;
  wire [11:0] debug_cpu_pc;
  wire [3:0] debug_cpu_accumulator;
  wire debug_cpu_carry;
  wire [2:0] debug_cpu_phase;
  wire debug_wmp_strobe;
  wire [3:0] debug_wmp_data;
  wire debug_rom_io_rd;
  wire debug_uart_rx_ready;

  mcs4_system_sim_top #(
    .CLOCKS_PER_BIT(SIM_CLOCKS_PER_BIT),
    .PHI_HALF(SIM_PHI_HALF),
    .ROM_INIT_FILE("monitor_rom.hex")
  ) dut (
    .sys_clk(sys_clk),
    .rst(rst),
    .test_in(test_in),
    .uart_rx(uart_rx),
    .uart_tx(uart_tx),
    .led_d3(led_d3),
    .debug_phi1(debug_phi1),
    .debug_phi2(debug_phi2),
    .debug_bus_data(debug_bus_data),
    .debug_cpu_data_oe(debug_cpu_data_oe),
    .debug_rom_data_oe(debug_rom_data_oe),
    .debug_ram_data_oe(debug_ram_data_oe),
    .debug_cm_rom(debug_cm_rom),
    .debug_cm_ram(debug_cm_ram),
    .debug_cpu_pc(debug_cpu_pc),
    .debug_cpu_accumulator(debug_cpu_accumulator),
    .debug_cpu_carry(debug_cpu_carry),
    .debug_cpu_phase(debug_cpu_phase),
    .debug_wmp_strobe(debug_wmp_strobe),
    .debug_wmp_data(debug_wmp_data),
    .debug_rom_io_rd(debug_rom_io_rd),
    .debug_uart_rx_ready(debug_uart_rx_ready)
  );

  wire phi1 = debug_phi1;
  wire phi2 = debug_phi2;
  wire wmp_strobe = debug_wmp_strobe;
  wire [3:0] wmp_data = debug_wmp_data;
  wire [3:0] bus_data = debug_bus_data;

  integer wmp_count;
  integer system_clock_count;
  reg saw_nonidle_bus;

  initial begin
    sys_clk = 1'b0;
    forever #5 sys_clk = ~sys_clk;
  end

  initial begin
    rst = 1'b1;
    test_in = 1'b0;
    uart_rx = 1'b1;
    wmp_count = 0;
    system_clock_count = 0;
    saw_nonidle_bus = 1'b0;

    repeat (8) @(posedge sys_clk);
    rst = 1'b0;
    repeat (MAX_SYSTEM_CLOCKS) @(posedge sys_clk);

    if (wmp_count < 2)
      $fatal(1, "system did not issue two monitor-ROM WMP operations");
    if (!saw_nonidle_bus)
      $fatal(1, "system bus remained at its inactive pull-up value");

    $display("SYSTEM_TEST_PASS wmp_count=%0d clocks=%0d", wmp_count, system_clock_count);
    $finish;
  end

  always @(posedge sys_clk) begin
    if (phi1 && phi2)
      $fatal(1, "phi1 and phi2 overlap");
    if ((debug_cpu_data_oe + debug_rom_data_oe + debug_ram_data_oe) > 1)
      $fatal(1, "multiple FPGA bus producers asserted");
    if (!rst) begin
      system_clock_count <= system_clock_count + 1;
      if (bus_data != 4'hF)
        saw_nonidle_bus <= 1'b1;
      if (wmp_strobe) begin
        wmp_count <= wmp_count + 1;
        $display("WMP value=%h time=%0t", wmp_data, $time);
      end
    end
  end

  initial begin
    #(MAX_SYSTEM_CLOCKS * 20);
    $fatal(1, "system test exceeded its wall-clock timeout");
  end

endmodule
