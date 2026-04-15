// UART Bridge: i4001 ROM I/O Port <-> 8-bit UART
//
// WHY: The 4004 CPU communicates via 4-bit nibbles on the i4001 I/O port.
//      The host terminal speaks 8-bit UART bytes. This bridge translates.
// WHAT: TX path: CPU writes nibble to I/O port -> bridge assembles two
//       nibbles into a byte -> sends via UART TX.
//       RX path: UART RX receives byte -> bridge splits into nibbles ->
//       presents on I/O port for CPU to read.
// HOW: The CPU uses WMP (Write Memory Port, opcode E1h) to write nibbles
//      to the i4001 I/O port. Bit 3 of the nibble is a control flag:
//        [3]=0, [2:0]=low 3 bits  -> latch as low nibble
//        [3]=1, [2:0]=high 3 bits -> combine with latched low, send byte
//      For RX, the bridge presents the received byte as two nibbles on
//      io_in, with a ready flag.

module uart_bridge #(
  parameter CLOCKS_PER_BIT = 234
)(
  input  wire       clk,
  input  wire       rst,

  // i4001 I/O port interface
  input  wire [3:0] io_out,     // from CPU via i4001 (WMP writes here)
  input  wire       io_wr,      // write strobe from i4001
  output reg  [3:0] io_in,      // to CPU via i4001 (CPU reads with RDR)

  // UART pins
  output wire       uart_tx,
  input  wire       uart_rx
);

  // --- TX path ---

  reg [7:0] tx_byte;
  reg       tx_send;
  wire      tx_ready;
  reg [3:0] tx_low_nibble;
  reg       tx_low_valid;

  // Edge detect io_wr (crosses from phi clock domain to sys_clk domain)
  reg [2:0] io_wr_sync;
  wire io_wr_pulse = io_wr_sync[1] && !io_wr_sync[2]; // rising edge

  always @(posedge clk) begin
    if (rst)
      io_wr_sync <= 3'd0;
    else
      io_wr_sync <= {io_wr_sync[1:0], io_wr};
  end

  uart_tx #(
    .CLOCKS_PER_BIT(CLOCKS_PER_BIT)
  ) u_tx (
    .clk(clk),
    .rst(rst),
    .tx_data(tx_byte),
    .tx_valid(tx_send),
    .tx_ready(tx_ready),
    .tx_out(uart_tx)
  );

  // Nibble assembly for TX
  // Protocol: CPU alternates low/high nibble writes via WMP.
  //   1st WMP: io_out = char[3:0] (low nibble, latched)
  //   2nd WMP: io_out = char[7:4] (high nibble, combined and sent)
  // The toggle flip-flop (tx_phase) tracks which nibble is next.
  reg tx_phase; // 0 = expecting low nibble, 1 = expecting high nibble

  always @(posedge clk) begin
    if (rst) begin
      tx_send <= 1'b0;
      tx_low_valid <= 1'b0;
      tx_low_nibble <= 4'd0;
      tx_phase <= 1'b0;
    end else begin
      tx_send <= 1'b0; // default: no send

      if (io_wr_pulse) begin
        if (!tx_phase) begin
          // Low nibble: latch all 4 bits
          tx_low_nibble <= io_out;
          tx_low_valid <= 1'b1;
          tx_phase <= 1'b1;
        end else begin
          // High nibble: combine with latched low, send byte
          if (tx_low_valid && tx_ready) begin
            tx_byte <= {io_out[3:0], tx_low_nibble[3:0]};
            tx_send <= 1'b1;
            tx_low_valid <= 1'b0;
          end
          tx_phase <= 1'b0;
        end
      end
    end
  end

  // --- RX path ---

  wire [7:0] rx_byte;
  wire       rx_valid;
  reg  [7:0] rx_buf;
  reg        rx_has_data;
  reg        rx_reading_high;  // 0 = present low nibble, 1 = present high

  uart_rx #(
    .CLOCKS_PER_BIT(CLOCKS_PER_BIT)
  ) u_rx (
    .clk(clk),
    .rst(rst),
    .rx_in(uart_rx),
    .rx_data(rx_byte),
    .rx_valid(rx_valid)
  );

  // RX nibble presentation
  // io_in[3] = data ready flag
  // io_in[2:0] = nibble data
  // CPU reads low nibble first, then reads high nibble
  always @(posedge clk) begin
    if (rst) begin
      rx_has_data <= 1'b0;
      rx_reading_high <= 1'b0;
      rx_buf <= 8'd0;
      io_in <= 4'd0;
    end else begin
      if (rx_valid) begin
        rx_buf <= rx_byte;
        rx_has_data <= 1'b1;
        rx_reading_high <= 1'b0;
      end

      if (rx_has_data) begin
        if (!rx_reading_high) begin
          io_in <= {1'b1, rx_buf[2:0]}; // ready + low 3 bits
        end else begin
          io_in <= {1'b1, rx_buf[6:4]}; // ready + high 3 bits
        end
      end else begin
        io_in <= 4'd0; // no data ready
      end

      // When CPU reads (detected by io_wr as an acknowledge mechanism),
      // advance to next nibble or clear
      if (io_wr_pulse && rx_has_data) begin
        if (!rx_reading_high) begin
          rx_reading_high <= 1'b1;
        end else begin
          rx_has_data <= 1'b0;
          rx_reading_high <= 1'b0;
        end
      end
    end
  end

endmodule
