// Proxy wrapper for resource estimation on devices without OSCH.
// Uses external clock input instead of internal oscillator.
// Only for synthesis estimation; not for actual board deployment.

module mcs4_proxy (
  input  wire clk_in,     // External clock input
  output wire uart_tx,
  input  wire uart_rx,
  output wire led_d3,
  input  wire btn_s1,
  input  wire btn_s2
);

  wire sys_clk;
  wire phi1, phi2;

  clock_gen #(
    .HALF_PERIOD(9),
    .FREQ_DIV(8),
    .USE_EXT_CLK(1)
  ) u_clk (
    .ext_clk(clk_in),
    .sys_clk(sys_clk),
    .phi1(phi1),
    .phi2(phi2),
    .rst_btn_n(btn_s1)
  );

  // Reuse mcs4_top internals without the clock_gen
  // For estimation purposes, just instantiate the data path

  reg [2:0] rst_sync;
  wire rst = rst_sync[2];
  always @(posedge sys_clk)
    rst_sync <= {rst_sync[1:0], ~btn_s1};

  wire [7:0] rom_addr;
  wire [7:0] rom_data;
  rom_bsram #(.INIT_FILE("monitor_rom.hex")) u_rom_mem (
    .clk(sys_clk), .addr(rom_addr), .data(rom_data));

  wire [7:0] ram_addr;
  wire [3:0] ram_rdata, ram_wdata;
  wire ram_we;
  ram_bsram u_ram_mem (
    .clk(sys_clk), .addr(ram_addr), .wdata(ram_wdata),
    .rdata(ram_rdata), .we(ram_we));

  wire [3:0] cpu_data_out, rom_data_out, ram_data_out;
  wire cpu_data_oe, rom_data_oe, ram_data_oe;
  wire cpu_sync, cpu_cm_rom, cpu_cm_ram;
  wire [3:0] bus_data = cpu_data_oe ? cpu_data_out :
                        rom_data_oe ? rom_data_out :
                        ram_data_oe ? ram_data_out : 4'hF;

  i4004_fpga u_cpu (
    .sys_clk(sys_clk), .phi1(phi1), .phi2(phi2), .rst(rst),
    .data_in(bus_data), .data_out(cpu_data_out), .data_oe(cpu_data_oe),
    .sync(cpu_sync), .cm_rom(cpu_cm_rom), .cm_ram(cpu_cm_ram), .test(1'b0));

  wire [3:0] rom_io_out, rom_io_in;
  wire rom_io_wr;
  i4001_fpga u_rom (
    .sys_clk(sys_clk), .phi1(phi1), .phi2(phi2), .rst(rst),
    .data_in(bus_data), .data_out(rom_data_out), .data_oe(rom_data_oe),
    .cm_rom(cpu_cm_rom), .sync(cpu_sync),
    .rom_addr(rom_addr), .rom_data(rom_data),
    .io_out(rom_io_out), .io_in(rom_io_in), .io_wr(rom_io_wr));

  i4002_fpga u_ram (
    .sys_clk(sys_clk), .phi1(phi1), .phi2(phi2), .rst(rst),
    .data_in(bus_data), .data_out(ram_data_out), .data_oe(ram_data_oe),
    .cm_ram(cpu_cm_ram),
    .ram_addr(ram_addr), .ram_rdata(ram_rdata),
    .ram_wdata(ram_wdata), .ram_we(ram_we), .port_out());

  wire io_wr_gated = rom_io_wr && cpu_data_oe;
  uart_bridge #(.CLOCKS_PER_BIT(234)) u_uart (
    .clk(sys_clk), .rst(rst),
    .io_out(rom_io_out), .io_wr(io_wr_gated),
    .io_in(rom_io_in), .uart_tx(uart_tx), .uart_rx(uart_rx));

  reg [19:0] heartbeat_cnt;
  always @(posedge phi1)
    if (rst) heartbeat_cnt <= 0;
    else heartbeat_cnt <= heartbeat_cnt + 1;
  assign led_d3 = heartbeat_cnt[19];

endmodule
