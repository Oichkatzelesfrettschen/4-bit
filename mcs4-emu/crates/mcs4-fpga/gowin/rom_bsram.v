// ROM Storage using Gowin BSRAM (SP primitive)
//
// WHY: FPGA Block RAMs are more efficient than LUT-based reg arrays
//      for the 256x8 ROM storage needed by the i4001.
// WHAT: Wraps Gowin SP (single-port SRAM) as a read-only ROM.
//      Initialized from a hex file via $readmemh.
// HOW: Clocked at sys_clk for 1-cycle read latency. The phi clock
//      runs ~36x slower, so the BSRAM read completes well within
//      a single phi phase -- no pipeline stall needed.

module rom_bsram #(
  parameter ADDR_W = 8,
  parameter DATA_W = 8,
  parameter DEPTH  = 256,
  parameter INIT_FILE = "monitor_rom.hex"
)(
  input  wire              clk,     // sys_clk (fast)
  input  wire [ADDR_W-1:0] addr,
  output reg  [DATA_W-1:0] data
);

  // Storage array -- synthesis tool infers BSRAM on Gowin,
  // behavioral reg array on iverilog.
  reg [DATA_W-1:0] mem [0:DEPTH-1];

  // Initialize from hex file
`ifdef MCS4_VERILATOR_RUNTIME
  // The host simulator receives a temporary common-stimulus ROM through this plusarg.
  // Synthesis and Icarus retain the reviewed INIT_FILE parameter boundary.
  reg [8*1024-1:0] active_init_file;

  initial begin
    if ($value$plusargs("mcs4_rom_init=%s", active_init_file)) begin
      $readmemh(active_init_file, mem);
    end else begin
      $readmemh(INIT_FILE, mem);
    end
  end
`else
  initial begin
    $readmemh(INIT_FILE, mem);
  end
`endif

  // Synchronous read (1-cycle latency)
  always @(posedge clk) begin
    data <= mem[addr];
  end

endmodule
