// Shared MCS-4 system datapath.
//
// The board and simulation wrappers supply a proven system clock, reset, and
// two-phase timing. This module owns the CPU, ROM, RAM, bus arbitration, UART
// bridge, and observation signals without selecting a physical clock source.

module mcs4_system_core #(
  parameter CLOCKS_PER_BIT = 234,
  parameter ROM_INIT_FILE = "monitor_rom.hex"
)(
  input  wire       sys_clk,
  input  wire       rst,
  input  wire       phi1,
  input  wire       phi2,
  input  wire       test_in,
  input  wire       uart_rx,
  output wire       uart_tx,
  output wire       led_heartbeat,
  output wire [3:0] debug_bus_data,
  output wire       debug_cpu_data_oe,
  output wire       debug_rom_data_oe,
  output wire       debug_ram_data_oe,
  output wire       debug_bus_driven,
  output wire       debug_cm_rom,
  output wire       debug_cm_ram,
  output wire       debug_rom_selected,
  output wire       debug_ram_selected,
  output wire [11:0] debug_cpu_pc,
  output wire [3:0] debug_cpu_accumulator,
  output wire       debug_cpu_carry,
  output wire [2:0] debug_cpu_phase,
  output wire       debug_wmp_strobe,
  output wire [3:0] debug_wmp_data,
  output wire       debug_rom_io_rd,
  output wire       debug_uart_rx_ready
);

  wire [7:0] rom_addr;
  wire [7:0] rom_data;
  wire [7:0] ram_addr;
  wire [3:0] ram_rdata;
  wire [3:0] ram_wdata;
  wire       ram_we;

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

  wire [3:0] cpu_data_out;
  wire [3:0] rom_data_out;
  wire [3:0] ram_data_out;
  wire [3:0] bus_data;
  wire [3:0] rom_io_in;
  wire [3:0] unused_rom_io_out;
  wire       rom_io_rd;
  wire [3:0] unused_ram_port_out;
  wire [3:0] cpu_wmp_data;
  wire       unused_rom_io_wr;
  wire       cpu_data_oe;
  wire       rom_data_oe;
  wire       ram_data_oe;
  wire       rom_chip_selected;
  wire       ram_chip_selected;
  wire       unused_cpu_sync;
  wire       cpu_cm_rom;
  wire       cpu_cm_ram;
  wire [3:0] cpu_ram_bank_select;
  wire       cpu_src_active;
  wire       cpu_ram_io_active;
  wire [3:0] cpu_ram_io_opcode;
  wire       cpu_wmp_strobe;
  wire       uart_rx_ready;
  wire       cpu_test_in;
  wire [11:0] cpu_debug_pc;
  wire [3:0] cpu_debug_accumulator;
  wire       cpu_debug_carry;
  wire [2:0] cpu_debug_phase;
  wire       cpu_debug_src_drive;
  wire       bus_driven;
  wire       cpu_ram_write_operation;
  wire       cpu_ram_read_operation;
  wire       debug_cm_ram_completed;
  wire [3:0] resolved_bus_data;
  reg  [3:0] bus_observation_data;
  reg        phi2_d;
  wire       phi2_rise = phi2 && !phi2_d;

  assign cpu_test_in = test_in || uart_rx_ready;

  i4004_fpga u_cpu (
    .sys_clk(sys_clk),
    .phi1(phi1),
    .phi2(phi2),
    .rst(rst),
    .data_in(bus_data),
    .data_out(cpu_data_out),
    .data_oe(cpu_data_oe),
    .sync(unused_cpu_sync),
    .cm_rom(cpu_cm_rom),
    .cm_ram(cpu_cm_ram),
    .ram_bank_select(cpu_ram_bank_select),
    .src_active(cpu_src_active),
    .ram_io_active(cpu_ram_io_active),
    .ram_io_opcode(cpu_ram_io_opcode),
    .test(cpu_test_in),
    .wmp_strobe(cpu_wmp_strobe),
    .wmp_data(cpu_wmp_data),
    .debug_pc(cpu_debug_pc),
    .debug_accumulator(cpu_debug_accumulator),
    .debug_carry(cpu_debug_carry),
    .debug_phase(cpu_debug_phase),
    .debug_src_drive(cpu_debug_src_drive)
  );

  i4001_fpga u_rom (
    .sys_clk(sys_clk),
    .phi1(phi1),
    .phi2(phi2),
    .rst(rst),
    .data_in(bus_data),
    .data_out(rom_data_out),
    .data_oe(rom_data_oe),
    .cm_rom(cpu_cm_rom),
    .rom_addr(rom_addr),
    .rom_data(rom_data),
    .io_out(unused_rom_io_out),
    .io_in(rom_io_in),
    .io_wr(unused_rom_io_wr),
    .io_rd(rom_io_rd),
    .chip_selected(rom_chip_selected)
  );

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
    .ram_bank_select(cpu_ram_bank_select),
    .src_active(cpu_src_active),
    .ram_io_active(cpu_ram_io_active),
    .ram_io_opcode(cpu_ram_io_opcode),
    .ram_addr(ram_addr),
    .ram_rdata(ram_rdata),
    .ram_wdata(ram_wdata),
    .ram_we(ram_we),
    .port_out(unused_ram_port_out),
    .chip_selected(ram_chip_selected)
  );

  // FPGA fabrics implement the shared four-bit bus as an explicit priority mux.
  // Undriven intervals retain the most recently driven nibble, matching the
  // MCS-4 bus model used by both devices and trace capture.  bus_driven keeps
  // an actual zero separate from an idle retained zero.
  assign bus_driven = cpu_data_oe || rom_data_oe || ram_data_oe;
  assign resolved_bus_data = cpu_data_oe ? cpu_data_out :
                             rom_data_oe ? rom_data_out :
                             ram_data_oe ? ram_data_out :
                             4'd0;
  assign bus_data = bus_driven ? resolved_bus_data : bus_observation_data;
  assign cpu_ram_write_operation = cpu_ram_io_active && !cpu_ram_io_opcode[3];
  assign cpu_ram_read_operation = cpu_ram_io_active && cpu_ram_io_opcode[3];
  // The CPU phase register points at the following phase after phi1. This
  // diagnostic signal therefore describes the transfer completed by phi2.
  assign debug_cm_ram_completed =
      (cpu_debug_phase == 3'd7 && (cpu_debug_src_drive || cpu_ram_write_operation)) ||
      (cpu_debug_phase == 3'd0 && (cpu_debug_src_drive || cpu_ram_read_operation));

  always @(posedge sys_clk) begin
    if (rst) begin
      phi2_d <= 1'b0;
      bus_observation_data <= 4'd0;
    end else begin
      phi2_d <= phi2;
      if (phi2_rise && bus_driven)
        bus_observation_data <= resolved_bus_data;
    end
  end

  // The generated CPU supplies the exact WMP strobe and nibble. The i4001
  // latch records generic X3 writes and does not identify WMP specifically.
  uart_bridge #(
    .CLOCKS_PER_BIT(CLOCKS_PER_BIT)
  ) u_uart (
    .clk(sys_clk),
    .rst(rst),
    .io_out(cpu_wmp_data),
    .io_wr(cpu_wmp_strobe),
    .io_rd(rom_io_rd),
    .io_in(rom_io_in),
    .rx_ready(uart_rx_ready),
    .uart_tx(uart_tx),
    .uart_rx(uart_rx)
  );

  reg [19:0] heartbeat_count;

  always @(posedge phi1) begin
    if (rst)
      heartbeat_count <= 20'd0;
    else
      heartbeat_count <= heartbeat_count + 20'd1;
  end

  assign led_heartbeat = heartbeat_count[19];
  assign debug_bus_data = bus_observation_data;
  assign debug_cpu_data_oe = cpu_data_oe;
  assign debug_rom_data_oe = rom_data_oe;
  assign debug_ram_data_oe = ram_data_oe;
  assign debug_bus_driven = bus_driven;
  assign debug_cm_rom = cpu_cm_rom;
  assign debug_cm_ram = debug_cm_ram_completed;
  assign debug_rom_selected = rom_chip_selected;
  assign debug_ram_selected = ram_chip_selected;
  assign debug_cpu_pc = cpu_debug_pc;
  assign debug_cpu_accumulator = cpu_debug_accumulator;
  assign debug_cpu_carry = cpu_debug_carry;
  assign debug_cpu_phase = cpu_debug_phase;
  assign debug_wmp_strobe = cpu_wmp_strobe;
  assign debug_wmp_data = cpu_wmp_data;
  assign debug_rom_io_rd = rom_io_rd;
  assign debug_uart_rx_ready = uart_rx_ready;

endmodule
