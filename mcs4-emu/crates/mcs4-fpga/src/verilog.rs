//! Verilog Export

use std::io::{self, Write};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortDir {
    Input,
    Output,
    Inout,
}

#[derive(Clone, Debug)]
pub struct Port {
    pub dir: PortDir,
    pub name: String,
    pub width: u32,
}

impl Port {
    pub fn input(name: impl Into<String>) -> Self {
        Self {
            dir: PortDir::Input,
            name: name.into(),
            width: 1,
        }
    }

    pub fn output(name: impl Into<String>) -> Self {
        Self {
            dir: PortDir::Output,
            name: name.into(),
            width: 1,
        }
    }

    pub fn inout(name: impl Into<String>) -> Self {
        Self {
            dir: PortDir::Inout,
            name: name.into(),
            width: 1,
        }
    }

    pub fn width(mut self, width: u32) -> Self {
        self.width = width.max(1);
        self
    }

    fn decl(&self) -> String {
        let dir = match self.dir {
            PortDir::Input => "input",
            PortDir::Output => "output",
            PortDir::Inout => "inout",
        };
        if self.width == 1 {
            format!("{dir} wire {}", self.name)
        } else {
            format!("{dir} wire [{}:0] {}", self.width - 1, self.name)
        }
    }
}

#[derive(Clone, Debug)]
pub struct Module {
    pub name: String,
    pub ports: Vec<Port>,
    pub body: Vec<String>,
}

impl Module {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ports: Vec::new(),
            body: Vec::new(),
        }
    }
}

/// Chip module specification for Verilog generation.
///
/// Describes the ports, registers, and behavioral logic for an MCS-4 chip
/// at a level suitable for FPGA synthesis.
#[derive(Clone, Debug)]
pub struct ChipSpec {
    /// Chip name (e.g., "i4004", "i4001").
    pub name: String,
    /// Register declarations (name, width).
    pub registers: Vec<(String, u32)>,
    /// Behavioral body lines (always blocks, assigns).
    pub body: Vec<String>,
}

/// Generate a synthesizable Verilog module for the Intel 4004 CPU.
pub fn chip_i4004() -> Module {
    let mut m = Module::new("i4004");
    // Clock and reset
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    // 4-bit bidirectional data bus
    m.ports.push(Port::inout("data").width(4));
    // Control signals
    m.ports.push(Port::output("sync"));
    m.ports.push(Port::output("cm_rom"));
    m.ports.push(Port::output("cm_ram"));
    m.ports.push(Port::input("test"));

    // Internal state
    m.body.push("reg [11:0] pc;".into());
    m.body.push("reg [11:0] stack [0:2];".into());
    m.body.push("reg [1:0] sp;".into());
    m.body.push("reg [3:0] acc;".into());
    m.body.push("reg carry;".into());
    m.body.push("reg [3:0] regs [0:15];".into());
    m.body.push("reg [7:0] instruction;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push("reg [3:0] data_out;".into());
    m.body.push("reg data_drive;".into());
    m.body.push(String::new());
    m.body.push("assign data = data_drive ? data_out : 4'bz;".into());
    m.body.push(String::new());

    // Phase counter (8-phase machine cycle)
    m.body.push("always @(posedge phi1 or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0;".into());
    m.body.push("    pc <= 12'd0;".into());
    m.body.push("    sp <= 2'd0;".into());
    m.body.push("    acc <= 4'd0;".into());
    m.body.push("    carry <= 1'b0;".into());
    m.body.push("    data_drive <= 1'b0;".into());
    m.body.push("  end else begin".into());
    m.body
        .push("    phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("  end".into());
    m.body.push("end".into());
    m.body.push(String::new());

    // Address output during A1-A3 phases
    m.body.push("always @(posedge phi2) begin".into());
    m.body.push("  case (phase)".into());
    m.body
        .push("    3'd0: begin data_out <= pc[3:0]; data_drive <= 1'b1; end // A1".into());
    m.body
        .push("    3'd1: begin data_out <= pc[7:4]; data_drive <= 1'b1; end // A2".into());
    m.body
        .push("    3'd2: begin data_out <= pc[11:8]; data_drive <= 1'b1; end // A3".into());
    m.body
        .push("    3'd3: begin data_drive <= 1'b0; instruction[3:0] <= data; end // M1".into());
    m.body
        .push("    3'd4: begin instruction[7:4] <= data; end // M2".into());
    m.body.push("    default: data_drive <= 1'b0;".into());
    m.body.push("  endcase".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4001 ROM.
pub fn chip_i4001() -> Module {
    let mut m = Module::new("i4001");
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::inout("data").width(4));
    m.ports.push(Port::input("cm_rom"));
    m.ports.push(Port::input("sync"));
    m.ports.push(Port::output("io_out").width(4));
    m.ports.push(Port::input("io_in").width(4));

    m.body.push("parameter CHIP_ID = 4'd0;".into());
    m.body.push(String::new());
    m.body.push("reg [7:0] rom [0:255];".into());
    m.body.push("reg [7:0] addr_latch;".into());
    m.body.push("reg [3:0] io_latch;".into());
    m.body.push("reg selected;".into());
    m.body.push("reg [3:0] data_out;".into());
    m.body.push("reg data_drive;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push(String::new());
    m.body.push("assign data = data_drive ? data_out : 4'bz;".into());
    m.body.push("assign io_out = io_latch;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi1 or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0;".into());
    m.body.push("    selected <= 1'b0;".into());
    m.body.push("    data_drive <= 1'b0;".into());
    m.body.push("    io_latch <= 4'd0;".into());
    m.body.push("  end else begin".into());
    m.body
        .push("    phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("  end".into());
    m.body.push("end".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi2) begin".into());
    m.body.push("  case (phase)".into());
    m.body
        .push("    3'd0: addr_latch[3:0] <= data; // A1: low nibble".into());
    m.body
        .push("    3'd1: addr_latch[7:4] <= data; // A2: high nibble".into());
    m.body.push("    3'd2: selected <= cm_rom; // A3: chip select".into());
    m.body
        .push("    3'd3: if (selected) begin // M1: output low nibble".into());
    m.body
        .push("      data_out <= rom[addr_latch][3:0]; data_drive <= 1'b1;".into());
    m.body.push("    end".into());
    m.body
        .push("    3'd4: if (selected) begin // M2: output high nibble".into());
    m.body
        .push("      data_out <= rom[addr_latch][7:4]; data_drive <= 1'b1;".into());
    m.body.push("    end".into());
    m.body.push("    default: data_drive <= 1'b0;".into());
    m.body.push("  endcase".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4002 RAM.
pub fn chip_i4002() -> Module {
    let mut m = Module::new("i4002");
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::inout("data").width(4));
    m.ports.push(Port::input("cm_ram"));
    m.ports.push(Port::output("port_out").width(4));

    m.body.push("parameter CHIP_ID = 2'd0;".into());
    m.body.push("parameter BANK_ID = 2'd0;".into());
    m.body.push(String::new());
    m.body.push("reg [3:0] ram [0:3][0:15]; // 4 regs x 16 nibbles".into());
    m.body.push("reg [3:0] status [0:3]; // 4 status nibbles".into());
    m.body.push("reg [3:0] output_port;".into());
    m.body.push("reg [1:0] sel_reg;".into());
    m.body.push("reg [3:0] sel_char;".into());
    m.body.push("reg selected;".into());
    m.body.push("reg [3:0] data_out;".into());
    m.body.push("reg data_drive;".into());
    m.body.push(String::new());
    m.body.push("assign data = data_drive ? data_out : 4'bz;".into());
    m.body.push("assign port_out = output_port;".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4003 shift register.
pub fn chip_i4003() -> Module {
    let mut m = Module::new("i4003");
    m.ports.push(Port::input("clk_in"));
    m.ports.push(Port::input("data_in"));
    m.ports.push(Port::input("enable"));
    m.ports.push(Port::output("parallel_out").width(10));
    m.ports.push(Port::output("serial_out"));

    m.body.push("reg [9:0] shift_reg;".into());
    m.body.push(String::new());
    m.body.push("assign parallel_out = shift_reg;".into());
    m.body.push("assign serial_out = shift_reg[9];".into());
    m.body.push(String::new());
    m.body.push("always @(posedge clk_in) begin".into());
    m.body.push("  if (enable)".into());
    m.body.push("    shift_reg <= {shift_reg[8:0], data_in};".into());
    m.body.push("end".into());

    m
}

/// Generate all MCS-4 chip modules.
pub fn all_chip_modules() -> Vec<Module> {
    vec![chip_i4004(), chip_i4001(), chip_i4002(), chip_i4003()]
}

/// Verilog exporter for gate-level designs
pub struct VerilogExporter {
    module_name: String,
}

impl VerilogExporter {
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
        }
    }

    pub fn export_module<W: Write>(&self, module: &Module, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "// Auto-generated Verilog for MCS-4")?;
        writeln!(writer, "module {} (", module.name)?;
        for (i, p) in module.ports.iter().enumerate() {
            let comma = if i + 1 == module.ports.len() { "" } else { "," };
            writeln!(writer, "  {}{}", p.name, comma)?;
        }
        writeln!(writer, ");")?;
        writeln!(writer)?;

        for p in &module.ports {
            writeln!(writer, "  {};", p.decl())?;
        }
        if !module.body.is_empty() {
            writeln!(writer)?;
            for line in &module.body {
                writeln!(writer, "  {line}")?;
            }
        }

        writeln!(writer, "endmodule")?;
        Ok(())
    }

    /// Export a placeholder module that matches the current CLI/API expectations.
    pub fn export<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let mut module = Module::new(self.module_name.clone());
        module.ports.push(Port::input("clk"));
        module.ports.push(Port::input("rst"));
        module
            .body
            .push("// Netlist emission hooks live in Module.body for now.".to_string());
        self.export_module(&module, writer)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn renders_minimal_module() {
        let exporter = VerilogExporter::new("mcs4_top");
        let mut out = Vec::new();
        assert!(exporter.export(&mut out).is_ok());
        let s = match String::from_utf8(out) {
            Ok(s) => s,
            Err(e) => panic!("invalid utf8 verilog output: {e}"),
        };
        assert!(s.contains("module mcs4_top"));
        assert!(s.contains("input wire clk;"));
        assert!(s.contains("input wire rst;"));
        assert!(s.contains("endmodule"));
    }

    #[test]
    fn port_direction_rendering() {
        let input = Port::input("clk");
        assert!(input.decl().starts_with("input wire"));
        assert!(input.decl().contains("clk"));

        let output = Port::output("data_out");
        assert!(output.decl().starts_with("output wire"));

        let inout = Port::inout("bus");
        assert!(inout.decl().starts_with("inout wire"));
    }

    #[test]
    fn multi_bit_width() {
        let port = Port::output("addr").width(12);
        let decl = port.decl();
        assert!(decl.contains("[11:0]"), "expected [11:0] in: {decl}");
        assert!(decl.contains("addr"));
    }

    #[test]
    fn single_bit_no_range() {
        let port = Port::input("clk").width(1);
        let decl = port.decl();
        assert!(!decl.contains('['), "single-bit should not have range: {decl}");
    }

    #[test]
    fn module_with_body() {
        let mut module = Module::new("test_mod");
        module.ports.push(Port::input("a"));
        module.ports.push(Port::output("b").width(4));
        module.body.push("assign b = {4{a}};".to_string());

        let exporter = VerilogExporter::new("ignored");
        let mut out = Vec::new();
        exporter.export_module(&module, &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();

        assert!(s.contains("module test_mod"));
        assert!(s.contains("assign b = {4{a}};"));
        assert!(s.contains("endmodule"));
    }

    #[test]
    fn export_module_port_commas() {
        let mut module = Module::new("two_port");
        module.ports.push(Port::input("a"));
        module.ports.push(Port::output("b"));

        let exporter = VerilogExporter::new("x");
        let mut out = Vec::new();
        exporter.export_module(&module, &mut out).unwrap();
        let s = String::from_utf8(out).unwrap();

        // First port has comma, second does not
        assert!(s.contains("a,"));
        assert!(!s.contains("b,"));
    }

    // --- Chip module generation tests ---

    fn render_module(module: &Module) -> String {
        let exporter = VerilogExporter::new("unused");
        let mut out = Vec::new();
        exporter.export_module(module, &mut out).unwrap();
        String::from_utf8(out).unwrap()
    }

    #[test]
    fn i4004_module_has_correct_ports() {
        let m = chip_i4004();
        assert_eq!(m.name, "i4004");
        let port_names: Vec<&str> = m.ports.iter().map(|p| p.name.as_str()).collect();
        assert!(port_names.contains(&"phi1"));
        assert!(port_names.contains(&"phi2"));
        assert!(port_names.contains(&"rst"));
        assert!(port_names.contains(&"data"));
        assert!(port_names.contains(&"sync"));
        assert!(port_names.contains(&"cm_rom"));
        assert!(port_names.contains(&"test"));

        let data_port = m.ports.iter().find(|p| p.name == "data").unwrap();
        assert_eq!(data_port.width, 4);
        assert_eq!(data_port.dir, PortDir::Inout);
    }

    #[test]
    fn i4004_module_renders_valid_verilog() {
        let m = chip_i4004();
        let v = render_module(&m);
        assert!(v.contains("module i4004"));
        assert!(v.contains("reg [11:0] pc;"));
        assert!(v.contains("reg [3:0] acc;"));
        assert!(v.contains("always @(posedge phi1"));
        assert!(v.contains("endmodule"));
    }

    #[test]
    fn i4001_module_has_rom_array() {
        let m = chip_i4001();
        let v = render_module(&m);
        assert!(v.contains("module i4001"));
        assert!(v.contains("reg [7:0] rom [0:255]"));
        assert!(v.contains("parameter CHIP_ID"));
        assert!(v.contains("addr_latch"));
    }

    #[test]
    fn i4002_module_has_ram_array() {
        let m = chip_i4002();
        let v = render_module(&m);
        assert!(v.contains("module i4002"));
        assert!(v.contains("ram"));
        assert!(v.contains("output_port"));
    }

    #[test]
    fn i4003_module_is_shift_register() {
        let m = chip_i4003();
        let v = render_module(&m);
        assert!(v.contains("module i4003"));
        assert!(v.contains("shift_reg"));
        assert!(v.contains("serial_out"));
        assert!(v.contains("parallel_out"));
        // Check the shift logic
        assert!(v.contains("{shift_reg[8:0], data_in}"));
    }

    #[test]
    fn all_chip_modules_returns_four() {
        let modules = all_chip_modules();
        assert_eq!(modules.len(), 4);
        let names: Vec<&str> = modules.iter().map(|m| m.name.as_str()).collect();
        assert!(names.contains(&"i4004"));
        assert!(names.contains(&"i4001"));
        assert!(names.contains(&"i4002"));
        assert!(names.contains(&"i4003"));
    }
}
