//! Validate the generated FPGA 4001 ROM-I/O timing contract with Icarus.

use std::{fs, process::Command};

use mcs4_fpga::{ChipTarget, ExportFlavor, ExportRequest, VerilogExporter};

fn tool_is_available(tool: &str) -> bool {
    Command::new(tool).arg("-V").output().is_ok()
}

fn testbench_source() -> &'static str {
    r#"
module tb_i4001_fpga_io;
  reg sys_clk = 1'b0;
  reg phi1 = 1'b0;
  reg phi2 = 1'b0;
  reg rst = 1'b1;
  reg [3:0] data_in = 4'd0;
  reg cm_rom = 1'b0;
  reg [7:0] rom_data = 8'd0;
  reg [3:0] io_in = 4'd0;
  wire [3:0] data_out;
  wire data_oe;
  wire [7:0] rom_addr;
  wire [3:0] io_out;
  wire io_wr;
  wire io_rd;

  i4001_fpga dut (
    .sys_clk(sys_clk), .phi1(phi1), .phi2(phi2), .rst(rst),
    .data_in(data_in), .data_out(data_out), .data_oe(data_oe),
    .cm_rom(cm_rom), .rom_addr(rom_addr), .rom_data(rom_data),
    .io_out(io_out), .io_in(io_in), .io_wr(io_wr), .io_rd(io_rd)
  );

  always #5 sys_clk = ~sys_clk;

  task pulse_phi2;
    begin
      @(negedge sys_clk); phi2 = 1'b1;
      @(negedge sys_clk); phi2 = 1'b0;
      #1;
    end
  endtask

  initial begin
    repeat (2) @(posedge sys_clk);
    rst = 1'b0;
    @(negedge sys_clk);

    // Latch a selected ROM instruction E2 (WRR) from the ROM source.
    dut.selected = 1'b1;
    dut.phase = 3'd3;
    rom_data = 8'hE2;
    pulse_phi2;
    dut.phase = 3'd4;
    pulse_phi2;

    // WRR captures the X2 bus nibble and emits the write pulse.
    dut.phase = 3'd6;
    data_in = 4'hA;
    pulse_phi2;
    if (io_out !== 4'hA || io_wr !== 1'b1)
      $fatal(1, "WRR did not update the selected ROM I/O latch");

    // RDR drives io_in only in selected X3.
    dut.instruction = 8'hEA;
    dut.phase = 3'd7;
    io_in = 4'hC;
    #1;
    if (data_oe !== 1'b1 || data_out !== 4'hC)
      $fatal(1, "RDR did not drive the selected ROM input nibble in X3");
    if (io_rd !== 1'b1)
      $fatal(1, "RDR did not expose its selected read observation");

    // WMP does not write the ROM I/O latch.
    dut.instruction = 8'hE1;
    dut.phase = 3'd6;
    data_in = 4'h5;
    pulse_phi2;
    if (io_out !== 4'hA || io_wr !== 1'b0)
      $fatal(1, "WMP incorrectly changed the ROM I/O latch");

    // An unselected RDR never drives the shared bus.
    dut.selected = 1'b0;
    dut.instruction = 8'hEA;
    dut.phase = 3'd7;
    #1;
    if (data_oe !== 1'b0)
      $fatal(1, "unselected ROM drove the shared bus");

    $display("I4001_FPGA_IO_TEST_PASS");
    $finish;
  end
endmodule
"#
}

#[test]
fn fpga_i4001_wrr_and_rdr_follow_the_selected_x2_x3_contract() {
    if !tool_is_available("iverilog") || !tool_is_available("vvp") {
        eprintln!("skipping 4001 FPGA I/O HDL test because Icarus Verilog is unavailable");
        return;
    }

    let temporary = tempfile::tempdir().expect("create temporary HDL directory");
    let module_path = temporary.path().join("i4001_fpga.v");
    let testbench_path = temporary.path().join("tb_i4001_fpga_io.v");
    let executable_path = temporary.path().join("tb_i4001_fpga_io");
    let module = VerilogExporter
        .module_for(ExportRequest::new(ChipTarget::I4001, ExportFlavor::Fpga))
        .expect("FPGA i4001 module exists");
    let mut module_file = fs::File::create(&module_path).expect("create generated i4001 module");
    VerilogExporter
        .export_module(&module, &mut module_file)
        .expect("render generated i4001 module");
    module_file.sync_all().expect("sync generated i4001 module");
    fs::write(&testbench_path, testbench_source()).expect("write I/O testbench");

    let compile = Command::new("iverilog")
        .args([
            "-g2012",
            "-Wall",
            "-Wno-timescale",
            "-s",
            "tb_i4001_fpga_io",
            "-o",
            executable_path.to_str().expect("temporary path is UTF-8"),
            testbench_path.to_str().expect("temporary path is UTF-8"),
            module_path.to_str().expect("temporary path is UTF-8"),
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
        String::from_utf8_lossy(&run.stdout).contains("I4001_FPGA_IO_TEST_PASS"),
        "I4001 testbench did not report its success marker"
    );
}
