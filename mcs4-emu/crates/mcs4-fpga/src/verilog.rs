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
}
