// MCS-4 Clock Generator for Gowin GW1N-2
//
// WHY: The 4004 needs non-overlapping phi1/phi2 clocks at ~750 kHz.
//      The GW1N-2 has no external crystal on the KIWI board; use OSCH.
// WHAT: OSCH -> sys_clk (~27 MHz) -> divider -> phi1/phi2 (~750 kHz)
// HOW: Counter divides sys_clk by 36 (4 states x 9 counts per state)
//      to produce non-overlapping phi1/phi2 with dead time.
//      Define SIM for iverilog simulation (behavioral oscillator).
//      For devices without OSCH, use ext_clk input instead.

module clock_gen #(
  parameter HALF_PERIOD = 9,
  parameter FREQ_DIV = 8,
  parameter USE_EXT_CLK = 0   // 1 = use ext_clk input, 0 = use OSCH
)(
  input  wire ext_clk,    // External clock (used when USE_EXT_CLK=1)
  output wire sys_clk,
  output wire phi1,
  output wire phi2,
  input  wire rst_btn_n
);

  localparam CNT_W = $clog2(2 * HALF_PERIOD);

  wire osc_out;

`ifdef SIM
  // Behavioral oscillator for iverilog simulation
  reg sim_clk = 0;
  always #18 sim_clk = ~sim_clk; // ~27.78 MHz
  assign osc_out = sim_clk;
`else
  generate
    if (USE_EXT_CLK) begin : gen_ext
      assign osc_out = ext_clk;
    end else begin : gen_osch
      // GW1N-2 has NO on-chip oscillator (OSC/OSCH only on GW1N-4+).
      // Use a LUT-based ring oscillator as a free-running clock source.
      // Frequency is approximate (~20-40 MHz depending on routing) but
      // sufficient for MCS-4 which only needs ~1 MHz phi clocks.
      // The UART baud rate will need calibration against actual frequency.
      (* keep = "true" *)
      wire [4:0] ring;
      assign ring[0] = ~ring[4];
      assign ring[1] = ~ring[0];
      assign ring[2] = ~ring[1];
      assign ring[3] = ~ring[2];
      assign ring[4] = ~ring[3];
      assign osc_out = ring[4];
    end
  endgenerate
`endif

  assign sys_clk = osc_out;

  // Reset synchronizer (active-low button -> active-high internal)
  reg [2:0] rst_sync;
  wire rst = rst_sync[2];

  always @(posedge osc_out) begin
    rst_sync <= {rst_sync[1:0], ~rst_btn_n};
  end

  // Two-phase clock generator with dead time
  //   State 0: dead time (both low)
  //   State 1: phi1 high
  //   State 2: dead time (both low)
  //   State 3: phi2 high
  reg [1:0] state;
  reg [CNT_W-1:0] cnt;
  reg phi1_r, phi2_r;

  assign phi1 = phi1_r;
  assign phi2 = phi2_r;

  always @(posedge osc_out) begin
    if (rst) begin
      state <= 2'd0;
      cnt <= {CNT_W{1'b0}};
      phi1_r <= 1'b0;
      phi2_r <= 1'b0;
    end else begin
      if (cnt == HALF_PERIOD - 1) begin
        cnt <= {CNT_W{1'b0}};
        state <= state + 2'd1;
      end else begin
        cnt <= cnt + 1'b1;
      end

      case (state)
        2'd0: begin phi1_r <= 1'b0; phi2_r <= 1'b0; end
        2'd1: begin phi1_r <= 1'b1; phi2_r <= 1'b0; end
        2'd2: begin phi1_r <= 1'b0; phi2_r <= 1'b0; end
        2'd3: begin phi1_r <= 1'b0; phi2_r <= 1'b1; end
      endcase
    end
  end

endmodule
