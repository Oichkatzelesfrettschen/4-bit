// Blink test for KIWI FPGA TINY 1P5 EVK (GW1N-2 LQFP100)
//
// WHY: Verify Gowin toolchain and board connectivity end-to-end before
//      building the full MCS-4 system.
// WHAT: Blinks LED D3 at ~1 Hz using internal oscillator (OSCH).
// HOW: gw_sh gowin/blink_test.tcl && openFPGALoader -c gwu2x build/blink_test.fs

module blink_test (
  output wire led_d3,
  input  wire btn_s1
);

  wire osc_clk;

  // Gowin internal oscillator: FREQ_DIV selects frequency.
  // FREQ_DIV=10 -> ~2.5 MHz (approximate, +/-5% tolerance)
`ifdef SIM
  // Behavioral stub for simulation
  reg sim_clk = 0;
  always #200 sim_clk = ~sim_clk; // ~2.5 MHz
  assign osc_clk = sim_clk;
`else
  OSCH #(
    .FREQ_DIV(10)
  ) osc_inst (
    .OSCOUT(osc_clk)
  );
`endif

  // Reset synchronizer (btn_s1 active-low)
  reg [1:0] rst_sync;
  wire rst = rst_sync[1];

  always @(posedge osc_clk) begin
    rst_sync <= {rst_sync[0], ~btn_s1};
  end

  // Counter: ~2.5 MHz / 2^21 ~ 1.2 Hz toggle
  reg [20:0] counter;

  always @(posedge osc_clk) begin
    if (rst)
      counter <= 21'd0;
    else
      counter <= counter + 21'd1;
  end

  assign led_d3 = counter[20];

endmodule
