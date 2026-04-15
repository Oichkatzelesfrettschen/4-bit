// RAM Storage using Gowin BSRAM (SP primitive)
//
// WHY: The i4002 RAM (4 registers x 16 nibbles = 64 x 4-bit) fits
//      in a single BSRAM block, freeing LUTs for logic.
// WHAT: Wraps Gowin SP as a synchronous read/write RAM.
// HOW: Single-port: read and write share the same address port.
//      Write-first mode: data written appears on output next cycle.

module ram_bsram #(
  parameter ADDR_W = 8,
  parameter DATA_W = 4,
  parameter DEPTH  = 256
)(
  input  wire              clk,
  input  wire [ADDR_W-1:0] addr,
  input  wire [DATA_W-1:0] wdata,
  output reg  [DATA_W-1:0] rdata,
  input  wire              we
);

  reg [DATA_W-1:0] mem [0:DEPTH-1];

  // Initialize to zero
  integer i;
  initial begin
    for (i = 0; i < DEPTH; i = i + 1)
      mem[i] = {DATA_W{1'b0}};
  end

  // Synchronous read/write
  always @(posedge clk) begin
    if (we) begin
      mem[addr] <= wdata;
      rdata <= wdata; // write-first
    end else begin
      rdata <= mem[addr];
    end
  end

endmodule
