// UART bridge between the MCS-4 console convention and an 8-bit UART.
//
// WMP writes form console transmit bytes. RDR reads receive-byte nibbles
// through the selected 4001. The bridge drives rx_ready to the CPU TEST input
// so both RDR operations retain all four payload bits.

module uart_bridge #(
  parameter CLOCKS_PER_BIT = 234
)(
  input  wire       clk,
  input  wire       rst,
  input  wire [3:0] io_out,
  input  wire       io_wr,
  input  wire       io_rd,
  output wire [3:0] io_in,
  output wire       rx_ready,
  output wire       uart_tx,
  input  wire       uart_rx
);

  reg [7:0] tx_byte;
  reg       tx_send;
  wire      tx_ready;
  reg [3:0] tx_low_nibble;
  reg       tx_low_valid;
  reg       tx_phase;
  reg [2:0] io_wr_sync;
  wire      io_wr_pulse = io_wr_sync[1] && !io_wr_sync[2];

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

  always @(posedge clk) begin
    if (rst) begin
      tx_send <= 1'b0;
      tx_low_valid <= 1'b0;
      tx_low_nibble <= 4'd0;
      tx_phase <= 1'b0;
    end else begin
      tx_send <= 1'b0;
      if (io_wr_pulse) begin
        if (!tx_phase) begin
          tx_low_nibble <= io_out;
          tx_low_valid <= 1'b1;
          tx_phase <= 1'b1;
        end else begin
          if (tx_low_valid && tx_ready) begin
            tx_byte <= {io_out, tx_low_nibble};
            tx_send <= 1'b1;
            tx_low_valid <= 1'b0;
          end
          tx_phase <= 1'b0;
        end
      end
    end
  end

  wire [7:0] rx_byte;
  wire       rx_valid;
  reg  [7:0] rx_buf;
  reg        rx_has_data;
  reg        rx_reading_high;
  reg        io_rd_d;
  wire       io_rd_pulse = io_rd && !io_rd_d;

  assign rx_ready = rx_has_data;
  assign io_in = rx_has_data ? (rx_reading_high ? rx_buf[7:4] : rx_buf[3:0]) : 4'd0;

  uart_rx #(
    .CLOCKS_PER_BIT(CLOCKS_PER_BIT)
  ) u_rx (
    .clk(clk),
    .rst(rst),
    .rx_in(uart_rx),
    .rx_data(rx_byte),
    .rx_valid(rx_valid)
  );

  always @(posedge clk) begin
    if (rst) begin
      rx_buf <= 8'd0;
      rx_has_data <= 1'b0;
      rx_reading_high <= 1'b0;
      io_rd_d <= 1'b0;
    end else begin
      io_rd_d <= io_rd;
      if (rx_valid) begin
        rx_buf <= rx_byte;
        rx_has_data <= 1'b1;
        rx_reading_high <= 1'b0;
      end else if (io_rd_pulse && rx_has_data) begin
        if (rx_reading_high) begin
          rx_has_data <= 1'b0;
          rx_reading_high <= 1'b0;
        end else begin
          rx_reading_high <= 1'b1;
        end
      end
    end
  end

endmodule
