//! Validate the UART bridge receive-byte and RDR handshake contract.

use std::{fs, path::PathBuf, process::Command};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("crate remains nested under the repository root")
        .to_path_buf()
}

fn tool_is_available(tool: &str) -> bool {
    Command::new(tool).arg("-V").output().is_ok()
}

fn testbench_source() -> &'static str {
    r#"
module tb_uart_bridge;
  localparam CLOCKS_PER_BIT = 4;
  reg clk = 1'b0;
  reg rst = 1'b1;
  reg [3:0] io_out = 4'd0;
  reg io_wr = 1'b0;
  reg io_rd = 1'b0;
  reg uart_rx = 1'b1;
  wire [3:0] io_in;
  wire rx_ready;
  wire uart_tx;

  uart_bridge #(.CLOCKS_PER_BIT(CLOCKS_PER_BIT)) dut (
    .clk(clk), .rst(rst), .io_out(io_out), .io_wr(io_wr), .io_rd(io_rd),
    .io_in(io_in), .rx_ready(rx_ready), .uart_tx(uart_tx), .uart_rx(uart_rx)
  );

  always #5 clk = ~clk;

  task send_uart_bit(input value);
    begin
      uart_rx = value;
      repeat (CLOCKS_PER_BIT) @(posedge clk);
    end
  endtask

  task send_uart_byte(input [7:0] value);
    integer bit_index;
    begin
      send_uart_bit(1'b0);
      for (bit_index = 0; bit_index < 8; bit_index = bit_index + 1)
        send_uart_bit(value[bit_index]);
      send_uart_bit(1'b1);
    end
  endtask

  task pulse_io_rd;
    begin
      @(negedge clk); io_rd = 1'b1;
      @(negedge clk); io_rd = 1'b0;
      #1;
    end
  endtask

  initial begin
    repeat (4) @(posedge clk);
    rst = 1'b0;
    repeat (2) @(posedge clk);

    send_uart_byte(8'hA5);
    repeat (8) @(posedge clk);
    if (rx_ready !== 1'b1 || io_in !== 4'h5)
      $fatal(1, "UART byte low nibble is not available to the first RDR");

    pulse_io_rd;
    if (rx_ready !== 1'b1 || io_in !== 4'hA)
      $fatal(1, "second RDR does not receive the UART byte high nibble");

    pulse_io_rd;
    if (rx_ready !== 1'b0 || io_in !== 4'h0)
      $fatal(1, "RDR acknowledgement does not retire the buffered UART byte");

    $display("UART_BRIDGE_TEST_PASS");
    $finish;
  end
endmodule
"#
}

#[test]
fn uart_rx_preserves_all_eight_bits_across_two_rdr_operations() {
    if !tool_is_available("iverilog") || !tool_is_available("vvp") {
        eprintln!("skipping UART bridge HDL test because Icarus Verilog is unavailable");
        return;
    }

    let root = repository_root();
    let temporary = tempfile::tempdir().expect("create temporary HDL directory");
    let testbench_path = temporary.path().join("tb_uart_bridge.v");
    let executable_path = temporary.path().join("tb_uart_bridge");
    fs::write(&testbench_path, testbench_source()).expect("write UART bridge testbench");

    let compile = Command::new("iverilog")
        .args([
            "-g2012",
            "-Wall",
            "-Wno-timescale",
            "-s",
            "tb_uart_bridge",
            "-o",
            executable_path.to_str().expect("temporary path is UTF-8"),
            testbench_path.to_str().expect("temporary path is UTF-8"),
            root.join("mcs4-emu/crates/mcs4-fpga/gowin/uart_hw.v")
                .to_str()
                .expect("UART hardware path is UTF-8"),
            root.join("mcs4-emu/crates/mcs4-fpga/gowin/uart_bridge.v")
                .to_str()
                .expect("UART bridge path is UTF-8"),
        ])
        .output()
        .expect("launch Icarus Verilog");
    assert!(
        compile.status.success(),
        "Icarus compile failed:\n{}",
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new("vvp")
        .arg(&executable_path)
        .output()
        .expect("launch Icarus runtime");
    assert!(
        run.status.success(),
        "Icarus execution failed:\n{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("UART_BRIDGE_TEST_PASS"),
        "UART bridge testbench did not report its success marker"
    );
}
