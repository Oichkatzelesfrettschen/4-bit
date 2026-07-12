//! Cross-check the source-located 4003 behavioral vectors against Rust and RTL.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use mcs4_chips::i4003::I4003;
use mcs4_fpga::{ChipTarget, ExportFlavor, ExportRequest, VerilogExporter};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Corpus {
    schema_version: u32,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
struct Scenario {
    name: String,
    source_locator: String,
    operations: Vec<Operation>,
    expect: Expectation,
}

#[derive(Debug, Deserialize)]
struct Expectation {
    parallel_out: u16,
    serial_out: bool,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Operation {
    SetEnableN { value: bool },
    SetData { value: bool },
    PulseCp,
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("crate remains nested under the repository root")
        .to_path_buf()
}

fn load_corpus() -> Corpus {
    let path = repository_root().join("docs/evidence/i4003_behavior_vectors.json");
    serde_json::from_slice(&fs::read(path).expect("read 4003 behavioral corpus")).expect("parse 4003 behavioral corpus")
}

fn tool_is_available(tool: &str) -> bool {
    Command::new(tool).arg("-V").output().is_ok()
}

fn rust_result(scenario: &Scenario) -> (u16, bool) {
    let mut chip = I4003::new();
    for operation in &scenario.operations {
        match operation {
            Operation::SetEnableN { value } => chip.set_enable_pin(*value),
            Operation::SetData { value } => chip.set_data_in(*value),
            Operation::PulseCp => {
                chip.set_clock(false);
                chip.set_clock(true);
            }
        }
    }
    (chip.parallel_out(), chip.serial_out())
}

fn testbench_source(scenario: &Scenario) -> String {
    let mut operations = String::new();
    for operation in &scenario.operations {
        match operation {
            Operation::SetEnableN { value } => {
                operations.push_str(&format!("  enable_n = 1'b{}; #1;\n", u8::from(*value)));
            }
            Operation::SetData { value } => {
                operations.push_str(&format!("  data_in = 1'b{}; #1;\n", u8::from(*value)));
            }
            Operation::PulseCp => operations.push_str("  pulse_cp;\n"),
        }
    }
    format!(
        "module tb_i4003_behavior;\n\
         reg clk_in;\n\
         reg data_in;\n\
         reg enable_n;\n\
         wire [9:0] parallel_out;\n\
         wire serial_out;\n\
         i4003 dut (.clk_in(clk_in), .data_in(data_in), .enable_n(enable_n), .parallel_out(parallel_out), .serial_out(serial_out));\n\
         task pulse_cp; begin\n\
           clk_in = 1'b0; #1;\n\
           clk_in = 1'b1; #1;\n\
         end endtask\n\
         initial begin\n\
           clk_in = 1'b0; data_in = 1'b0; enable_n = 1'b0; #1;\n\
         {operations}\
           $display(\"RESULT %0d %0d\", parallel_out, serial_out);\n\
           $finish;\n\
         end\n\
         endmodule\n"
    )
}

fn rtl_result(tempdir: &Path, module_path: &Path, scenario: &Scenario) -> (u16, bool) {
    let testbench_path = tempdir.join(format!("tb_{}.v", scenario.name));
    let executable_path = tempdir.join(format!("tb_{}", scenario.name));
    fs::write(&testbench_path, testbench_source(scenario)).expect("write generated 4003 testbench");

    let compile = Command::new("iverilog")
        .args([
            "-g2012",
            "-Wall",
            "-s",
            "tb_i4003_behavior",
            "-o",
            executable_path.to_str().expect("temporary path is UTF-8"),
            testbench_path.to_str().expect("temporary path is UTF-8"),
            module_path.to_str().expect("temporary path is UTF-8"),
        ])
        .output()
        .expect("launch Icarus Verilog");
    assert!(
        compile.status.success(),
        "Icarus compile failed for {}:\n{}",
        scenario.name,
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new("vvp")
        .arg(&executable_path)
        .output()
        .expect("launch Icarus runtime");
    assert!(
        run.status.success(),
        "Icarus run failed for {}:\n{}",
        scenario.name,
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    let result_line = stdout
        .lines()
        .find(|line| line.starts_with("RESULT "))
        .expect("testbench reports a RESULT line");
    let values: Vec<u16> = result_line
        .split_whitespace()
        .skip(1)
        .map(|value| value.parse::<u16>().expect("RESULT fields are decimal values"))
        .collect();
    assert_eq!(values.len(), 2, "RESULT line has parallel and serial values");
    (values[0], values[1] != 0)
}

#[test]
#[allow(clippy::unwrap_used)]
fn behavioral_vectors_agree_with_rust_and_icarus() {
    if !tool_is_available("iverilog") || !tool_is_available("vvp") {
        eprintln!("skipping 4003 RTL vector test because Icarus Verilog is unavailable");
        return;
    }

    let corpus = load_corpus();
    assert_eq!(corpus.schema_version, 1);
    let tempdir = tempfile::tempdir().expect("create temporary HDL directory");
    let module_path = tempdir.path().join("i4003.v");
    let module = VerilogExporter
        .module_for(ExportRequest::new(ChipTarget::I4003, ExportFlavor::Behavioral))
        .expect("behavioral i4003 module exists");
    let mut module_file = fs::File::create(&module_path).expect("create behavioral i4003 module");
    VerilogExporter
        .export_module(&module, &mut module_file)
        .expect("render behavioral i4003 module");
    module_file.sync_all().expect("sync behavioral i4003 module");

    for scenario in &corpus.scenarios {
        let locator_path = scenario
            .source_locator
            .split(':')
            .next()
            .expect("source locator has path");
        assert!(
            repository_root().join(locator_path).is_file(),
            "missing source locator for {}",
            scenario.name
        );
        let rust = rust_result(scenario);
        let rtl = rtl_result(tempdir.path(), &module_path, scenario);
        let expected = (scenario.expect.parallel_out, scenario.expect.serial_out);
        assert_eq!(rust, expected, "Rust result differs for {}", scenario.name);
        assert_eq!(rtl, expected, "RTL result differs for {}", scenario.name);
    }
}
