// MCS-4 System Testbench
//
// WHY: Verify the integrated MCS-4 system fetches instructions from ROM
//      and outputs the greeting via UART TX before committing to FPGA.
// WHAT: Instantiates mcs4_top with SIM-mode clock, monitors UART TX
//      output, captures transmitted bytes.
// HOW: iverilog -DSIM -o tb gowin/tb_mcs4_system.v gowin/mcs4_top.v \
//        gowin/clock_gen.v gowin/uart_hw.v gowin/uart_bridge.v \
//        gowin/rom_bsram.v gowin/ram_bsram.v \
//        build/i4004_fpga.v build/i4001_fpga.v build/i4002_fpga.v
//      vvp tb

`timescale 1ns / 1ps

module tb_mcs4_system;

  // ============================================================
  // Clock and Reset
  // ============================================================

  reg btn_s1;  // active-low reset button
  reg btn_s2;
  wire uart_tx;
  wire led_d3;

  // For simulation, use faster UART (fewer clocks per bit)
  // and shorter phi half-period for faster simulation
  localparam SIM_CLOCKS_PER_BIT = 4;  // very fast for sim
  localparam SIM_PHI_HALF = 2;        // 2 sys_clk per phi half

  mcs4_top #(
    .CLOCKS_PER_BIT(SIM_CLOCKS_PER_BIT),
    .PHI_HALF(SIM_PHI_HALF),
    .ROM_INIT_FILE("../gowin/monitor_rom.hex")
  ) dut (
    .uart_tx(uart_tx),
    .uart_rx(1'b1),  // idle high (no input)
    .led_d3(led_d3),
    .btn_s1(btn_s1),
    .btn_s2(btn_s2)
  );

  // ============================================================
  // UART TX Monitor
  // ============================================================
  //
  // Captures bytes transmitted on uart_tx by sampling at baud rate.

  reg [7:0] captured_byte;
  reg [3:0] bit_count;
  integer byte_count;

  // Access internal sys_clk for timing reference
  wire sys_clk = dut.sys_clk;
  wire phi1 = dut.phi1;
  wire phi2 = dut.phi2;
  wire rst = dut.rst;

  // Monitor task: detect start bit, sample 8 data bits, verify stop
  task automatic uart_receive;
    output [7:0] data;
    integer i;
    begin
      // Wait for start bit (falling edge on uart_tx)
      @(negedge uart_tx);

      // Wait half a bit period to center on start bit
      repeat (SIM_CLOCKS_PER_BIT / 2) @(posedge sys_clk);

      // Verify start bit is still low
      if (uart_tx !== 1'b0) begin
        $display("ERROR: False start bit detected");
        data = 8'hFF;
        disable uart_receive;
      end

      // Sample 8 data bits (LSB first)
      for (i = 0; i < 8; i = i + 1) begin
        repeat (SIM_CLOCKS_PER_BIT) @(posedge sys_clk);
        data[i] = uart_tx;
      end

      // Wait for stop bit
      repeat (SIM_CLOCKS_PER_BIT) @(posedge sys_clk);
      if (uart_tx !== 1'b1) begin
        $display("WARNING: Framing error (stop bit not high)");
      end
    end
  endtask

  // ============================================================
  // Waveform Dump
  // ============================================================

  initial begin
    $dumpfile("tb_mcs4_system.vcd");
    $dumpvars(0, tb_mcs4_system);
  end

  // ============================================================
  // Test Stimulus
  // ============================================================

  initial begin
    btn_s1 = 1'b0;  // assert reset (active-low)
    btn_s2 = 1'b1;  // user button released
    byte_count = 0;

    // Hold reset for a while
    #500;
    btn_s1 = 1'b1;  // release reset

    $display("=== MCS-4 System Testbench ===");
    $display("Reset released at time %0t", $time);
    $display("Waiting for UART output...");
    $display("");
  end

  // ============================================================
  // UART Capture Process
  // ============================================================

  reg [7:0] greeting [0:15];  // capture up to 16 bytes
  integer gi;

  initial begin
    // Wait for reset release
    #600;

    // Try to capture greeting bytes
    for (gi = 0; gi < 10; gi = gi + 1) begin
      uart_receive(captured_byte);
      greeting[gi] = captured_byte;
      byte_count = byte_count + 1;
      $display("UART RX byte %0d: 0x%02h '%c' at time %0t",
               gi, captured_byte,
               (captured_byte >= 8'h20 && captured_byte < 8'h7F) ? captured_byte : 8'h2E,
               $time);
    end

    $display("");
    $display("=== Captured %0d bytes ===", byte_count);

    // Verify greeting starts with 'M'
    if (byte_count > 0 && greeting[0] == 8'h4D) begin
      $display("PASS: First byte is 'M' (0x4D)");
    end else if (byte_count > 0) begin
      $display("INFO: First byte is 0x%02h (expected 0x4D for 'M')", greeting[0]);
    end else begin
      $display("WARNING: No bytes captured");
    end

    $display("=== Testbench complete ===");
    $finish;
  end

  // ============================================================
  // Timeout Watchdog
  // ============================================================

  initial begin
    #10_000_000;  // 10 ms timeout
    $display("TIMEOUT: No UART output after 10 ms simulation time");
    $display("=== Testbench timed out ===");
    $finish;
  end

  // ============================================================
  // Debug: Monitor CPU Phase and Bus Activity
  // ============================================================

  // Debug WMP strobe
  wire wmp_s = dut.cpu_wmp_strobe;
  wire [3:0] wmp_d = dut.cpu_wmp_data;
  wire [3:0] cpu_acc = dut.u_cpu.acc;
  reg [7:0] wmp_count = 0;
  always @(posedge sys_clk) begin
    if (wmp_s && wmp_count < 20) begin
      $display("  WMP #%0d: data=%h acc=%h at time %0t", wmp_count, wmp_d, cpu_acc, $time);
      wmp_count <= wmp_count + 1;
    end
  end

  // Debug registers
  wire need_op = dut.u_cpu.need_operand;
  wire [7:0] operand_reg = dut.u_cpu.operand;
  wire [3:0] r2 = dut.u_cpu.regs[2];
  wire [3:0] r3 = dut.u_cpu.regs[3];

  wire [2:0] cpu_phase = dut.u_cpu.phase;
  wire [11:0] cpu_pc = dut.u_cpu.pc;
  wire [7:0] cpu_instr = dut.u_cpu.instruction;
  wire [3:0] bus = dut.bus_data;
  wire cpu_oe = dut.cpu_data_oe;
  wire rom_oe = dut.rom_data_oe;

  // Print first few instruction fetches
  reg [7:0] fetch_count;

  initial fetch_count = 0;

  always @(posedge phi2) begin
    if (!rst && cpu_phase == 3'd4 && fetch_count < 20) begin
      // End of M2 phase: instruction fetch complete
      #1;  // small delay to let signals settle
      $display("  Fetch #%0d: PC=0x%03h Instr=0x%02h bus=%b need_op=%b operand=0x%02h R2=%h R3=%h acc=%h",
               fetch_count, cpu_pc, cpu_instr, bus, need_op, operand_reg, r2, r3, cpu_acc);
      fetch_count <= fetch_count + 1;
    end
  end

endmodule
