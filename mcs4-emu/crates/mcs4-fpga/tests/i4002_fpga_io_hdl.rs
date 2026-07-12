//! Validate generated FPGA 4002 source, write, and read timing contracts.

use std::{fs, process::Command};

use mcs4_fpga::{ChipTarget, ExportFlavor, ExportRequest, VerilogExporter};

fn tool_is_available(tool: &str) -> bool {
    Command::new(tool).arg("-V").output().is_ok()
}

fn testbench_source() -> &'static str {
    r#"
module tb_i4002_fpga_io;
  reg sys_clk = 1'b0;
  reg phi1 = 1'b0;
  reg phi2 = 1'b0;
  reg rst = 1'b1;
  reg [3:0] data_in = 4'd0;
  reg cm_ram = 1'b0;
  reg [3:0] ram_bank_select = 4'b0001;
  reg src_active = 1'b0;
  reg ram_io_active = 1'b0;
  reg [3:0] ram_io_opcode = 4'd0;
  reg [3:0] ram_rdata = 4'd0;
  wire [3:0] data_out;
  wire data_oe;
  wire [7:0] ram_addr;
  wire [3:0] ram_wdata;
  wire ram_we;
  wire [3:0] port_out;

  i4002_fpga #(.CHIP_ID(2'd0), .BANK_ID(2'd0)) dut (
    .sys_clk(sys_clk), .phi1(phi1), .phi2(phi2), .rst(rst),
    .data_in(data_in), .data_out(data_out), .data_oe(data_oe), .cm_ram(cm_ram),
    .ram_bank_select(ram_bank_select), .src_active(src_active),
    .ram_io_active(ram_io_active), .ram_io_opcode(ram_io_opcode),
    .ram_addr(ram_addr), .ram_rdata(ram_rdata), .ram_wdata(ram_wdata),
    .ram_we(ram_we), .port_out(port_out)
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

    // SRC selects chip zero in bank zero and latches the address nibbles.
    src_active = 1'b1;
    dut.phase = 3'd6;
    data_in = 4'h0;
    pulse_phi2;
    dut.phase = 3'd7;
    data_in = 4'h3;
    pulse_phi2;
    if (dut.selected !== 1'b1 || ram_addr !== 8'h30)
      $fatal(1, "SRC did not select the target RAM chip and address");

    // WMP writes the RAM output port without asserting RAM write-enable.
    src_active = 1'b0;
    cm_ram = 1'b1;
    ram_io_active = 1'b1;
    ram_io_opcode = 4'h1;
    dut.phase = 3'd6;
    data_in = 4'hA;
    pulse_phi2;
    if (port_out !== 4'hA || ram_we !== 1'b0)
      $fatal(1, "WMP did not update only the RAM output port");

    // WRM presents the CPU nibble to the external BSRAM write interface.
    ram_io_opcode = 4'h0;
    dut.phase = 3'd6;
    data_in = 4'hB;
    pulse_phi2;
    if (ram_we !== 1'b1 || ram_wdata !== 4'hB)
      $fatal(1, "WRM did not drive the BSRAM write interface");

    // RDM and RD0 drive the correct external RAM and status values in X3.
    ram_io_opcode = 4'h9;
    ram_rdata = 4'hD;
    dut.phase = 3'd7;
    #1;
    if (data_oe !== 1'b1 || data_out !== 4'hD)
      $fatal(1, "RDM did not drive external RAM data in X3");

    ram_io_opcode = 4'h4;
    dut.phase = 3'd6;
    data_in = 4'hC;
    pulse_phi2;
    ram_io_opcode = 4'hC;
    dut.phase = 3'd7;
    #1;
    if (data_oe !== 1'b1 || data_out !== 4'hC)
      $fatal(1, "RD0 did not drive the stored status nibble in X3");

    ram_bank_select = 4'b0000;
    #1;
    if (data_oe !== 1'b0)
      $fatal(1, "unselected RAM drove the shared bus");

    $display("I4002_FPGA_IO_TEST_PASS");
    $finish;
  end
endmodule
"#
}

#[test]
fn fpga_i4002_source_and_io_operations_follow_x2_x3_contracts() {
    if !tool_is_available("iverilog") || !tool_is_available("vvp") {
        eprintln!("skipping 4002 FPGA I/O HDL test because Icarus Verilog is unavailable");
        return;
    }

    let temporary = tempfile::tempdir().expect("create temporary HDL directory");
    let module_path = temporary.path().join("i4002_fpga.v");
    let testbench_path = temporary.path().join("tb_i4002_fpga_io.v");
    let executable_path = temporary.path().join("tb_i4002_fpga_io");
    let module = VerilogExporter
        .module_for(ExportRequest::new(ChipTarget::I4002, ExportFlavor::Fpga))
        .expect("FPGA i4002 module exists");
    let mut module_file = fs::File::create(&module_path).expect("create generated i4002 module");
    VerilogExporter
        .export_module(&module, &mut module_file)
        .expect("render generated i4002 module");
    module_file.sync_all().expect("sync generated i4002 module");
    fs::write(&testbench_path, testbench_source()).expect("write I/O testbench");

    let compile = Command::new("iverilog")
        .args([
            "-g2012",
            "-Wall",
            "-Wno-timescale",
            "-s",
            "tb_i4002_fpga_io",
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
        String::from_utf8_lossy(&run.stdout).contains("I4002_FPGA_IO_TEST_PASS"),
        "I4002 testbench did not report its success marker"
    );
}
