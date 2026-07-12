// MCS-4 two-phase clock divider.
//
// This module derives non-overlapping phi1 and phi2 from a supplied system
// clock. It does not select, synthesize, or estimate a board clock source.

module clock_gen #(
  parameter HALF_PERIOD = 9
)(
  input  wire sys_clk,
  input  wire rst,
  output wire phi1,
  output wire phi2
);

  localparam COUNTER_WIDTH = (HALF_PERIOD <= 1) ? 1 : $clog2(HALF_PERIOD);
  localparam [COUNTER_WIDTH-1:0] HALF_PERIOD_LAST = COUNTER_WIDTH'(HALF_PERIOD - 1);

  reg [1:0] state;
  reg [COUNTER_WIDTH-1:0] phase_count;
  reg phi1_r;
  reg phi2_r;

  assign phi1 = phi1_r;
  assign phi2 = phi2_r;

  always @(posedge sys_clk) begin
    if (rst) begin
      state <= 2'd0;
      phase_count <= {COUNTER_WIDTH{1'b0}};
      phi1_r <= 1'b0;
      phi2_r <= 1'b0;
    end else if (phase_count == HALF_PERIOD_LAST) begin
      phase_count <= {COUNTER_WIDTH{1'b0}};
      case (state)
        2'd0: begin
          state <= 2'd1;
          phi1_r <= 1'b1;
          phi2_r <= 1'b0;
        end
        2'd1: begin
          state <= 2'd2;
          phi1_r <= 1'b0;
          phi2_r <= 1'b0;
        end
        2'd2: begin
          state <= 2'd3;
          phi1_r <= 1'b0;
          phi2_r <= 1'b1;
        end
        default: begin
          state <= 2'd0;
          phi1_r <= 1'b0;
          phi2_r <= 1'b0;
        end
      endcase
    end else begin
      phase_count <= phase_count + 1'b1;
    end
  end

endmodule
