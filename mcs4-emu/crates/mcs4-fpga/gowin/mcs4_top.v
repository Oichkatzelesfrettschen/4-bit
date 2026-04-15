// MCS-4 System Top-Level for Gowin GW1N-2 LQFP100
//
// WHY: Integrates the i4004 CPU, i4001 ROM, i4002 RAM, clock generator,
//      UART bridge, and board peripherals into a complete MCS-4 computer.
// WHAT: Top-level module for FPGA synthesis. All chip-to-chip buses use
//      explicit priority multiplexing (NO internal tristates).
// HOW: Instantiates FPGA-safe chip variants (split bus) and wires them
//      through a data bus mux. OSCH provides system clock, counter
//      divider generates phi1/phi2.
//
// Architecture:
//   clock_gen -> phi1/phi2, sys_clk
//   i4004_fpga (CPU) <-> bus_mux <-> i4001_fpga (ROM) + i4002_fpga (RAM)
//   i4001 io_out -> uart_bridge -> uart_tx/rx
//   rom_bsram (monitor program storage)
//   ram_bsram (data memory)
//   LED heartbeat, button reset

module mcs4_top #(
  parameter CLOCKS_PER_BIT = 234,  // 27 MHz / 115200 = 234
  parameter PHI_HALF = 9,          // sys_clk cycles per phi half-period
  parameter ROM_INIT_FILE = "monitor_rom.hex"
)(
  // Board I/O
  output wire uart_tx,
  input  wire uart_rx,
  output wire led_d3,
  input  wire btn_s1,   // reset (active-low)
  input  wire btn_s2    // user (active-low)
);

  // ============================================================
  // Clock Generation
  // ============================================================

  wire sys_clk;
  wire phi1, phi2;

  clock_gen #(
    .HALF_PERIOD(PHI_HALF),
    .FREQ_DIV(8)    // ~27 MHz
  ) u_clk (
    .ext_clk(1'b0), // unused: OSCH provides clock
    .sys_clk(sys_clk),
    .phi1(phi1),
    .phi2(phi2),
    .rst_btn_n(btn_s1)
  );

  // Synchronized reset (active-high, from clock_gen's rst_sync)
  reg [2:0] rst_sync;
  wire rst = rst_sync[2];

  always @(posedge sys_clk) begin
    rst_sync <= {rst_sync[1:0], ~btn_s1};
  end

  // ============================================================
  // ROM Storage (BSRAM)
  // ============================================================

  wire [7:0] rom_addr;
  wire [7:0] rom_data;

  rom_bsram #(
    .ADDR_W(8),
    .DATA_W(8),
    .DEPTH(256),
    .INIT_FILE(ROM_INIT_FILE)
  ) u_rom_mem (
    .clk(sys_clk),
    .addr(rom_addr),
    .data(rom_data)
  );

  // ============================================================
  // RAM Storage (BSRAM)
  // ============================================================

  wire [7:0] ram_addr;
  wire [3:0] ram_rdata;
  wire [3:0] ram_wdata;
  wire       ram_we;

  ram_bsram #(
    .ADDR_W(8),
    .DATA_W(4),
    .DEPTH(256)
  ) u_ram_mem (
    .clk(sys_clk),
    .addr(ram_addr),
    .wdata(ram_wdata),
    .rdata(ram_rdata),
    .we(ram_we)
  );

  // ============================================================
  // CPU: i4004 (FPGA-safe split bus)
  // ============================================================

  wire [3:0] cpu_data_out;
  wire       cpu_data_oe;
  wire       cpu_sync;
  wire       cpu_cm_rom;
  wire       cpu_cm_ram;
  wire       cpu_wmp_strobe;
  wire [3:0] cpu_wmp_data;

  // Bus mux output -> CPU data input
  wire [3:0] bus_data;

  i4004_fpga u_cpu (
    .sys_clk(sys_clk),
    .phi1(phi1),
    .phi2(phi2),
    .rst(rst),
    .data_in(bus_data),
    .data_out(cpu_data_out),
    .data_oe(cpu_data_oe),
    .sync(cpu_sync),
    .cm_rom(cpu_cm_rom),
    .cm_ram(cpu_cm_ram),
    .test(1'b0),
    .wmp_strobe(cpu_wmp_strobe),
    .wmp_data(cpu_wmp_data)
  );

  // ============================================================
  // ROM: i4001 (FPGA-safe split bus + external BSRAM)
  // ============================================================

  wire [3:0] rom_data_out;
  wire       rom_data_oe;
  wire [3:0] rom_io_out;
  wire [3:0] rom_io_in;
  wire       rom_io_wr;

  i4001_fpga #(
    .CHIP_ID(4'd0)
  ) u_rom (
    .sys_clk(sys_clk),
    .phi1(phi1),
    .phi2(phi2),
    .rst(rst),
    .data_in(bus_data),
    .data_out(rom_data_out),
    .data_oe(rom_data_oe),
    .cm_rom(cpu_cm_rom),
    .sync(cpu_sync),
    .rom_addr(rom_addr),
    .rom_data(rom_data),
    .io_out(rom_io_out),
    .io_in(rom_io_in),
    .io_wr(rom_io_wr)
  );

  // ============================================================
  // RAM: i4002 (FPGA-safe split bus + external BSRAM)
  // ============================================================

  wire [3:0] ram_data_out;
  wire       ram_data_oe;

  i4002_fpga #(
    .CHIP_ID(2'd0),
    .BANK_ID(2'd0)
  ) u_ram (
    .sys_clk(sys_clk),
    .phi1(phi1),
    .phi2(phi2),
    .rst(rst),
    .data_in(bus_data),
    .data_out(ram_data_out),
    .data_oe(ram_data_oe),
    .cm_ram(cpu_cm_ram),
    .ram_addr(ram_addr),
    .ram_rdata(ram_rdata),
    .ram_wdata(ram_wdata),
    .ram_we(ram_we),
    .port_out()  // unused for now
  );

  // ============================================================
  // Data Bus Priority Multiplexer
  // ============================================================
  //
  // WHY: FPGA fabrics have no physical internal tristates. Each chip
  //      has data_out and data_oe. The bus mux selects which chip
  //      drives the shared bus based on output enable priority.
  // WHAT: CPU has highest priority (drives address during A1-A3),
  //      ROM drives during M1-M2, RAM drives during X phases.
  // HOW: Simple priority chain -- at most one driver is active per phase.

  assign bus_data = cpu_data_oe ? cpu_data_out :
                    rom_data_oe ? rom_data_out :
                    ram_data_oe ? ram_data_out :
                    4'hF;  // default pull-up (no driver)

  // ============================================================
  // UART Bridge (i4001 I/O port <-> serial)
  // ============================================================

  // WMP strobe comes directly from CPU -- clean registered pulse, no timing issues.
  uart_bridge #(
    .CLOCKS_PER_BIT(CLOCKS_PER_BIT)
  ) u_uart (
    .clk(sys_clk),
    .rst(rst),
    .io_out(cpu_wmp_data),  // CPU provides the WMP data directly
    .io_wr(cpu_wmp_strobe), // CPU provides the strobe directly
    .io_in(rom_io_in),
    .uart_tx(uart_tx),
    .uart_rx(uart_rx)
  );

  // ============================================================
  // LED Heartbeat
  // ============================================================
  //
  // Blink LED D3 at ~1 Hz to show CPU is alive.
  // Toggle on phi1 rising edge; counter rolls over at ~750k -> ~1 Hz

  reg [19:0] heartbeat_cnt;

  always @(posedge phi1) begin
    if (rst)
      heartbeat_cnt <= 20'd0;
    else
      heartbeat_cnt <= heartbeat_cnt + 20'd1;
  end

  assign led_d3 = heartbeat_cnt[19];

endmodule
