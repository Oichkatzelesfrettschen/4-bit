// Hardware UART TX + RX for MCS-4 FPGA System
//
// WHY: Provides serial communication between the MCS-4 system and a host
//      terminal via the GWU2U USB-UART bridge on the KIWI board.
// WHAT: Full-duplex UART with parameterized baud rate. 8N1 format.
// HOW: Oversample RX at 1x (edge-aligned). TX shifts out LSB-first.
//      CLOCKS_PER_BIT = sys_clk_freq / baud_rate.
//      At 27 MHz / 115200 = 234. At 27 MHz / 9600 = 2812.

module uart_tx #(
  parameter CLOCKS_PER_BIT = 234  // 27 MHz / 115200
)(
  input  wire       clk,
  input  wire       rst,
  input  wire [7:0] tx_data,
  input  wire       tx_valid,   // pulse high to start transmission
  output wire       tx_ready,   // high when idle, ready to accept data
  output wire       tx_out      // serial output (active high, idle high)
);

  localparam CNT_W = $clog2(CLOCKS_PER_BIT);

  // States
  localparam S_IDLE  = 2'd0;
  localparam S_START = 2'd1;
  localparam S_DATA  = 2'd2;
  localparam S_STOP  = 2'd3;

  reg [1:0]       state;
  reg [CNT_W-1:0] bit_timer;
  reg [2:0]       bit_idx;
  reg [7:0]       shift_reg;
  reg             tx_r;

  assign tx_out = tx_r;
  assign tx_ready = (state == S_IDLE);

  always @(posedge clk) begin
    if (rst) begin
      state <= S_IDLE;
      tx_r <= 1'b1; // idle high
      bit_timer <= {CNT_W{1'b0}};
      bit_idx <= 3'd0;
    end else begin
      case (state)
        S_IDLE: begin
          tx_r <= 1'b1;
          if (tx_valid) begin
            shift_reg <= tx_data;
            state <= S_START;
            bit_timer <= {CNT_W{1'b0}};
          end
        end

        S_START: begin
          tx_r <= 1'b0; // start bit
          if (bit_timer == CLOCKS_PER_BIT - 1) begin
            bit_timer <= {CNT_W{1'b0}};
            bit_idx <= 3'd0;
            state <= S_DATA;
          end else begin
            bit_timer <= bit_timer + 1'b1;
          end
        end

        S_DATA: begin
          tx_r <= shift_reg[0]; // LSB first
          if (bit_timer == CLOCKS_PER_BIT - 1) begin
            bit_timer <= {CNT_W{1'b0}};
            shift_reg <= {1'b0, shift_reg[7:1]};
            if (bit_idx == 3'd7) begin
              state <= S_STOP;
            end else begin
              bit_idx <= bit_idx + 3'd1;
            end
          end else begin
            bit_timer <= bit_timer + 1'b1;
          end
        end

        S_STOP: begin
          tx_r <= 1'b1; // stop bit
          if (bit_timer == CLOCKS_PER_BIT - 1) begin
            state <= S_IDLE;
            bit_timer <= {CNT_W{1'b0}};
          end else begin
            bit_timer <= bit_timer + 1'b1;
          end
        end
      endcase
    end
  end

endmodule


module uart_rx #(
  parameter CLOCKS_PER_BIT = 234  // 27 MHz / 115200
)(
  input  wire       clk,
  input  wire       rst,
  input  wire       rx_in,       // serial input
  output reg  [7:0] rx_data,     // received byte (valid when rx_valid pulses)
  output reg        rx_valid     // pulses high for 1 clk when byte received
);

  localparam CNT_W = $clog2(CLOCKS_PER_BIT);
  localparam HALF_BIT = CLOCKS_PER_BIT / 2;

  // States
  localparam S_IDLE  = 2'd0;
  localparam S_START = 2'd1;
  localparam S_DATA  = 2'd2;
  localparam S_STOP  = 2'd3;

  reg [1:0]       state;
  reg [CNT_W-1:0] bit_timer;
  reg [2:0]       bit_idx;
  reg [7:0]       shift_reg;

  // Input synchronizer (2-stage for metastability)
  reg [1:0] rx_sync;
  wire rx_s = rx_sync[1];

  always @(posedge clk) begin
    rx_sync <= {rx_sync[0], rx_in};
  end

  always @(posedge clk) begin
    if (rst) begin
      state <= S_IDLE;
      rx_valid <= 1'b0;
      bit_timer <= {CNT_W{1'b0}};
      bit_idx <= 3'd0;
      rx_data <= 8'd0;
    end else begin
      rx_valid <= 1'b0; // default: clear valid pulse

      case (state)
        S_IDLE: begin
          if (!rx_s) begin // falling edge: start bit detected
            state <= S_START;
            bit_timer <= {CNT_W{1'b0}};
          end
        end

        S_START: begin
          // Wait half a bit period to sample at center of start bit
          if (bit_timer == HALF_BIT - 1) begin
            if (!rx_s) begin // still low: valid start bit
              bit_timer <= {CNT_W{1'b0}};
              bit_idx <= 3'd0;
              state <= S_DATA;
            end else begin
              state <= S_IDLE; // false start, abort
            end
          end else begin
            bit_timer <= bit_timer + 1'b1;
          end
        end

        S_DATA: begin
          if (bit_timer == CLOCKS_PER_BIT - 1) begin
            bit_timer <= {CNT_W{1'b0}};
            shift_reg <= {rx_s, shift_reg[7:1]}; // LSB first
            if (bit_idx == 3'd7) begin
              state <= S_STOP;
            end else begin
              bit_idx <= bit_idx + 3'd1;
            end
          end else begin
            bit_timer <= bit_timer + 1'b1;
          end
        end

        S_STOP: begin
          if (bit_timer == CLOCKS_PER_BIT - 1) begin
            if (rx_s) begin // valid stop bit
              rx_data <= shift_reg;
              rx_valid <= 1'b1;
            end
            // Return to idle regardless (framing error silently dropped)
            state <= S_IDLE;
            bit_timer <= {CNT_W{1'b0}};
          end else begin
            bit_timer <= bit_timer + 1'b1;
          end
        end
      endcase
    end
  end

endmodule
