// External-clock LED test for the KIWI FPGA board.
//
// The module does not assume an internal oscillator exists on GW1N-2.

module blink_test (
  input  wire sys_clk_in,
  output wire led_d3,
  input  wire btn_s1
);

  reg [1:0] reset_sync;
  wire rst = reset_sync[1];
  reg [20:0] counter;

  always @(posedge sys_clk_in) begin
    reset_sync <= {reset_sync[0], ~btn_s1};
    if (rst)
      counter <= 21'd0;
    else
      counter <= counter + 21'd1;
  end

  assign led_d3 = counter[20];

endmodule
