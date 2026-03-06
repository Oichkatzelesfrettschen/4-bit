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

/// Generate a synthesizable Verilog module for the Intel 4008 address latch.
pub fn chip_i4008() -> Module {
    let mut m = Module::new("i4008");
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("data").width(4));
    m.ports.push(Port::input("cm_rom"));
    m.ports.push(Port::output("addr").width(12));
    m.ports.push(Port::output("addr_valid"));
    m.ports.push(Port::output("rom_sel").width(4));

    m.body.push("reg [11:0] addr_latch;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push("reg valid;".into());
    m.body.push("reg [3:0] rom_select;".into());
    m.body.push(String::new());
    m.body.push("assign addr = addr_latch;".into());
    m.body.push("assign addr_valid = valid;".into());
    m.body.push("assign rom_sel = rom_select;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi1 or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0;".into());
    m.body.push("    valid <= 1'b0;".into());
    m.body.push("    addr_latch <= 12'd0;".into());
    m.body.push("    rom_select <= 4'd0;".into());
    m.body.push("  end else begin".into());
    m.body
        .push("    phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("  end".into());
    m.body.push("end".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi2) begin".into());
    m.body.push("  case (phase)".into());
    m.body
        .push("    3'd0: begin addr_latch[3:0] <= data; valid <= 1'b0; end // A1".into());
    m.body.push("    3'd1: addr_latch[7:4] <= data; // A2".into());
    m.body.push(
        "    3'd2: begin addr_latch[11:8] <= data; valid <= 1'b1; rom_select <= {cm_rom, data[2:0]}; end // A3".into(),
    );
    m.body.push("    default: ;".into());
    m.body.push("  endcase".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4009 I/O expander.
pub fn chip_i4009() -> Module {
    let mut m = Module::new("i4009");
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::inout("data").width(4));
    m.ports.push(Port::input("cm_ram"));
    m.ports.push(Port::output("io_out").width(4));
    m.ports.push(Port::input("io_in").width(4));
    m.ports.push(Port::output("ram_bank").width(4));

    m.body.push("reg [3:0] out_latch;".into());
    m.body.push("reg [3:0] in_latch;".into());
    m.body.push("reg [3:0] bank_sel;".into());
    m.body.push("reg enabled;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push("reg [3:0] data_out;".into());
    m.body.push("reg data_drive;".into());
    m.body.push(String::new());
    m.body.push("assign data = data_drive ? data_out : 4'bz;".into());
    m.body.push("assign io_out = out_latch;".into());
    m.body.push("assign ram_bank = bank_sel;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi1 or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0;".into());
    m.body.push("    enabled <= 1'b0;".into());
    m.body.push("    data_drive <= 1'b0;".into());
    m.body.push("    out_latch <= 4'd0;".into());
    m.body.push("    bank_sel <= 4'd0;".into());
    m.body.push("  end else begin".into());
    m.body
        .push("    phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("  end".into());
    m.body.push("end".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi2) begin".into());
    m.body.push("  case (phase)".into());
    m.body
        .push("    3'd2: begin enabled <= cm_ram; bank_sel <= data; end // A3".into());
    m.body
        .push("    3'd5: if (enabled) out_latch <= data; // X2: latch output".into());
    m.body.push("    3'd6: if (enabled) begin // X3: drive input".into());
    m.body.push("      in_latch <= io_in;".into());
    m.body.push("      data_out <= io_in;".into());
    m.body.push("      data_drive <= 1'b1;".into());
    m.body.push("    end".into());
    m.body.push("    default: data_drive <= 1'b0;".into());
    m.body.push("  endcase".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 3216 bus driver (non-inverting).
pub fn chip_i3216() -> Module {
    let mut m = Module::new("i3216");
    m.ports.push(Port::inout("port_a").width(4));
    m.ports.push(Port::inout("port_b").width(4));
    m.ports.push(Port::input("dir")); // 1=A->B, 0=B->A
    m.ports.push(Port::input("cs_n")); // active low chip select

    m.body.push("// Non-inverting bidirectional buffer".into());
    m.body.push("assign port_b = (!cs_n && dir) ? port_a : 4'bz;".into());
    m.body.push("assign port_a = (!cs_n && !dir) ? port_b : 4'bz;".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 3226 bus driver (inverting).
pub fn chip_i3226() -> Module {
    let mut m = Module::new("i3226");
    m.ports.push(Port::inout("port_a").width(4));
    m.ports.push(Port::inout("port_b").width(4));
    m.ports.push(Port::input("dir")); // 1=A->B, 0=B->A
    m.ports.push(Port::input("cs_n")); // active low chip select

    m.body.push("// Inverting bidirectional buffer".into());
    m.body.push("assign port_b = (!cs_n && dir) ? ~port_a : 4'bz;".into());
    m.body.push("assign port_a = (!cs_n && !dir) ? ~port_b : 4'bz;".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4101 static RAM.
pub fn chip_i4101() -> Module {
    let mut m = Module::new("i4101");
    m.ports.push(Port::input("clk"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("addr").width(8));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::input("cs"));
    m.ports.push(Port::input("we"));
    m.ports.push(Port::input("oe"));

    m.body.push("reg [3:0] mem [0:255];".into());
    m.body.push("reg [3:0] out_reg;".into());
    m.body.push(String::new());
    m.body
        .push("assign data_out = (cs && oe && !we) ? out_reg : 4'bz;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge clk) begin".into());
    m.body.push("  if (cs && we)".into());
    m.body.push("    mem[addr] <= data_in;".into());
    m.body.push("  if (cs && oe && !we)".into());
    m.body.push("    out_reg <= mem[addr];".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4201 clock generator.
pub fn chip_i4201() -> Module {
    let mut m = Module::new("i4201");
    m.ports.push(Port::input("xtal"));
    m.ports.push(Port::input("rst_in"));
    m.ports.push(Port::input("stop_in"));
    m.ports.push(Port::output("phi1"));
    m.ports.push(Port::output("phi2"));
    m.ports.push(Port::output("rst_out"));
    m.ports.push(Port::output("stp_out"));

    m.body.push("reg phi1_r, phi2_r;".into());
    m.body.push("reg rst_out_r, stp_out_r;".into());
    m.body.push("reg [2:0] div_cnt; // divide-by-7 counter".into());
    m.body.push("reg phase_sel;".into());
    m.body.push(String::new());
    m.body.push("assign phi1 = phi1_r;".into());
    m.body.push("assign phi2 = phi2_r;".into());
    m.body.push("assign rst_out = rst_out_r;".into());
    m.body.push("assign stp_out = stp_out_r;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge xtal or posedge rst_in) begin".into());
    m.body.push("  if (rst_in) begin".into());
    m.body.push("    div_cnt <= 3'd0;".into());
    m.body.push("    phase_sel <= 1'b0;".into());
    m.body.push("    phi1_r <= 1'b0;".into());
    m.body.push("    phi2_r <= 1'b0;".into());
    m.body.push("    rst_out_r <= 1'b1;".into());
    m.body.push("    stp_out_r <= 1'b0;".into());
    m.body.push("  end else if (stop_in) begin".into());
    m.body.push("    phi1_r <= 1'b0;".into());
    m.body.push("    phi2_r <= 1'b0;".into());
    m.body.push("    stp_out_r <= 1'b1;".into());
    m.body.push("  end else begin".into());
    m.body.push("    rst_out_r <= 1'b0;".into());
    m.body.push("    stp_out_r <= 1'b0;".into());
    m.body.push("    if (div_cnt == 3'd6) begin".into());
    m.body.push("      div_cnt <= 3'd0;".into());
    m.body.push("      phase_sel <= ~phase_sel;".into());
    m.body.push("    end else begin".into());
    m.body.push("      div_cnt <= div_cnt + 3'd1;".into());
    m.body.push("    end".into());
    m.body.push("    // Non-overlapping: dead time at count 0".into());
    m.body.push("    phi1_r <= !phase_sel && (div_cnt > 3'd0);".into());
    m.body.push("    phi2_r <= phase_sel && (div_cnt > 3'd0);".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4289 memory interface.
pub fn chip_i4289() -> Module {
    let mut m = Module::new("i4289");
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::inout("data").width(4));
    m.ports.push(Port::input("cm_rom"));
    m.ports.push(Port::input("cm_ram"));
    m.ports.push(Port::output("mem_addr").width(8));
    m.ports.push(Port::output("mem_data").width(4));
    m.ports.push(Port::input("mem_in").width(4));
    m.ports.push(Port::output("oe_n"));
    m.ports.push(Port::output("we_n"));

    m.body.push("reg [11:0] pc_addr;".into());
    m.body.push("reg [7:0] src_latch;".into());
    m.body.push("reg [3:0] data_out;".into());
    m.body.push("reg data_drive;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push("reg cs_rom, cs_ram;".into());
    m.body.push("reg oe_n_r, we_n_r;".into());
    m.body.push(String::new());
    m.body.push("assign data = data_drive ? data_out : 4'bz;".into());
    m.body.push("assign mem_addr = src_latch;".into());
    m.body.push("assign mem_data = data_out;".into());
    m.body.push("assign oe_n = oe_n_r;".into());
    m.body.push("assign we_n = we_n_r;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi1 or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0;".into());
    m.body.push("    data_drive <= 1'b0;".into());
    m.body.push("    oe_n_r <= 1'b1;".into());
    m.body.push("    we_n_r <= 1'b1;".into());
    m.body.push("  end else begin".into());
    m.body
        .push("    phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("  end".into());
    m.body.push("end".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi2) begin".into());
    m.body.push("  case (phase)".into());
    m.body.push("    3'd0: pc_addr[3:0] <= data; // A1".into());
    m.body.push("    3'd1: pc_addr[7:4] <= data; // A2".into());
    m.body
        .push("    3'd2: begin pc_addr[11:8] <= data; cs_rom <= cm_rom; cs_ram <= cm_ram; end // A3".into());
    m.body
        .push("    3'd3: if (cs_rom) begin oe_n_r <= 1'b0; data_out <= mem_in; data_drive <= 1'b1; end // M1".into());
    m.body
        .push("    3'd4: if (cs_rom) begin data_out <= mem_in; end // M2".into());
    m.body
        .push("    3'd5: begin data_drive <= 1'b0; oe_n_r <= 1'b1; src_latch[3:0] <= data; end // X1".into());
    m.body.push("    3'd6: src_latch[7:4] <= data; // X2".into());
    m.body
        .push("    default: begin data_drive <= 1'b0; we_n_r <= 1'b1; end".into());
    m.body.push("  endcase".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4308 ROM.
pub fn chip_i4308() -> Module {
    let mut m = Module::new("i4308");
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::inout("data").width(4));
    m.ports.push(Port::input("cm_rom"));
    m.ports.push(Port::output("io_out").width(4));
    m.ports.push(Port::input("io_in").width(4));

    m.body.push("parameter CHIP_ID = 4'd0;".into());
    m.body.push(String::new());
    m.body.push("reg [7:0] rom [0:1023];".into());
    m.body.push("reg [9:0] addr_latch;".into());
    m.body.push("reg [3:0] io_ports [0:3];".into());
    m.body.push("reg selected;".into());
    m.body.push("reg [3:0] data_out;".into());
    m.body.push("reg data_drive;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push(String::new());
    m.body.push("assign data = data_drive ? data_out : 4'bz;".into());
    m.body.push("assign io_out = io_ports[0];".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi1 or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0;".into());
    m.body.push("    selected <= 1'b0;".into());
    m.body.push("    data_drive <= 1'b0;".into());
    m.body.push("  end else begin".into());
    m.body
        .push("    phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("  end".into());
    m.body.push("end".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi2) begin".into());
    m.body.push("  case (phase)".into());
    m.body.push("    3'd0: addr_latch[3:0] <= data; // A1".into());
    m.body.push("    3'd1: addr_latch[7:4] <= data; // A2".into());
    m.body
        .push("    3'd2: begin addr_latch[9:8] <= data[1:0]; selected <= cm_rom; end // A3".into());
    m.body
        .push("    3'd3: if (selected) begin data_out <= rom[addr_latch][3:0]; data_drive <= 1'b1; end // M1".into());
    m.body
        .push("    3'd4: if (selected) begin data_out <= rom[addr_latch][7:4]; end // M2".into());
    m.body.push("    default: data_drive <= 1'b0;".into());
    m.body.push("  endcase".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4207 crystal clock.
pub fn chip_i4207() -> Module {
    let mut m = Module::new("i4207");
    m.ports.push(Port::input("xtal_in"));
    m.ports.push(Port::output("clk_out"));

    m.body.push("// Single-phase crystal clock pass-through".into());
    m.body
        .push("// In real hardware, drives 4209 for two-phase generation".into());
    m.body.push("assign clk_out = xtal_in;".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4209 phase converter.
pub fn chip_i4209() -> Module {
    let mut m = Module::new("i4209");
    m.ports.push(Port::input("clk_in"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::output("phi1"));
    m.ports.push(Port::output("phi2"));

    m.body
        .push("reg [1:0] state; // 0=dead1, 1=phi1, 2=dead2, 3=phi2".into());
    m.body.push("reg phi1_r, phi2_r;".into());
    m.body.push(String::new());
    m.body.push("assign phi1 = phi1_r;".into());
    m.body.push("assign phi2 = phi2_r;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge clk_in or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    state <= 2'd0;".into());
    m.body.push("    phi1_r <= 1'b0;".into());
    m.body.push("    phi2_r <= 1'b0;".into());
    m.body.push("  end else begin".into());
    m.body.push("    state <= state + 2'd1;".into());
    m.body.push("    case (state)".into());
    m.body
        .push("      2'd0: begin phi1_r <= 1'b0; phi2_r <= 1'b0; end // dead time".into());
    m.body
        .push("      2'd1: begin phi1_r <= 1'b1; phi2_r <= 1'b0; end // phi1 high".into());
    m.body
        .push("      2'd2: begin phi1_r <= 1'b0; phi2_r <= 1'b0; end // dead time".into());
    m.body
        .push("      2'd3: begin phi1_r <= 1'b0; phi2_r <= 1'b1; end // phi2 high".into());
    m.body.push("    endcase".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4211 RC clock generator.
pub fn chip_i4211() -> Module {
    let mut m = Module::new("i4211");
    m.ports.push(Port::input("rc_osc")); // RC oscillator input
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::output("phi1"));
    m.ports.push(Port::output("phi2"));

    m.body.push("reg [1:0] phase_cnt;".into());
    m.body.push("reg phi1_r, phi2_r;".into());
    m.body.push(String::new());
    m.body.push("assign phi1 = phi1_r;".into());
    m.body.push("assign phi2 = phi2_r;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge rc_osc or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase_cnt <= 2'd0;".into());
    m.body.push("    phi1_r <= 1'b0;".into());
    m.body.push("    phi2_r <= 1'b0;".into());
    m.body.push("  end else begin".into());
    m.body.push("    phase_cnt <= phase_cnt + 2'd1;".into());
    m.body.push("    case (phase_cnt)".into());
    m.body
        .push("      2'd0: begin phi1_r <= 1'b1; phi2_r <= 1'b0; end".into());
    m.body
        .push("      2'd1: begin phi1_r <= 1'b0; phi2_r <= 1'b0; end // dead time".into());
    m.body
        .push("      2'd2: begin phi1_r <= 1'b0; phi2_r <= 1'b1; end".into());
    m.body
        .push("      2'd3: begin phi1_r <= 1'b0; phi2_r <= 1'b0; end // dead time".into());
    m.body.push("    endcase".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4265 programmable I/O.
pub fn chip_i4265() -> Module {
    let mut m = Module::new("i4265");
    m.ports.push(Port::input("clk"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("cs"));
    m.ports.push(Port::input("wr"));
    m.ports.push(Port::input("port_sel").width(2));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::input("dir_wr")); // direction register write
    m.ports.push(Port::inout("io_0").width(4));
    m.ports.push(Port::inout("io_1").width(4));
    m.ports.push(Port::inout("io_2").width(4));
    m.ports.push(Port::inout("io_3").width(4));

    m.body.push("reg [3:0] port_data [0:3];".into());
    m.body.push("reg [3:0] port_dir [0:3]; // 1=output, 0=input".into());
    m.body.push(String::new());

    m.body
        .push("// Output drivers: output bits drive the pin, input bits are high-Z".into());
    m.body.push("assign io_0 = port_dir[0] & port_data[0];".into());
    m.body.push("assign io_1 = port_dir[1] & port_data[1];".into());
    m.body.push("assign io_2 = port_dir[2] & port_data[2];".into());
    m.body.push("assign io_3 = port_dir[3] & port_data[3];".into());
    m.body.push(String::new());

    m.body
        .push("// Read mux: output bits read back written value, input bits read pin".into());
    m.body.push("wire [3:0] port_read [0:3];".into());
    m.body
        .push("assign port_read[0] = (port_dir[0] & port_data[0]) | (~port_dir[0] & io_0);".into());
    m.body
        .push("assign port_read[1] = (port_dir[1] & port_data[1]) | (~port_dir[1] & io_1);".into());
    m.body
        .push("assign port_read[2] = (port_dir[2] & port_data[2]) | (~port_dir[2] & io_2);".into());
    m.body
        .push("assign port_read[3] = (port_dir[3] & port_data[3]) | (~port_dir[3] & io_3);".into());
    m.body.push("assign data_out = port_read[port_sel];".into());
    m.body.push(String::new());

    m.body.push("always @(posedge clk or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    port_data[0] <= 4'd0; port_data[1] <= 4'd0;".into());
    m.body.push("    port_data[2] <= 4'd0; port_data[3] <= 4'd0;".into());
    m.body.push("    port_dir[0] <= 4'd0; port_dir[1] <= 4'd0;".into());
    m.body.push("    port_dir[2] <= 4'd0; port_dir[3] <= 4'd0;".into());
    m.body.push("  end else if (cs) begin".into());
    m.body.push("    if (dir_wr) port_dir[port_sel] <= data_in;".into());
    m.body.push("    else if (wr) port_data[port_sel] <= data_in;".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4316 LCD driver.
pub fn chip_i4316() -> Module {
    let mut m = Module::new("i4316");
    m.ports.push(Port::input("clk"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("cs"));
    m.ports.push(Port::input("wr"));
    m.ports.push(Port::input("digit_sel").width(4));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("seg_out").width(4));
    m.ports.push(Port::output("backplane"));

    m.body.push("reg [3:0] segments [0:15];".into());
    m.body.push("reg bp_phase;".into());
    m.body.push("reg [3:0] active_digit;".into());
    m.body.push(String::new());
    m.body
        .push("assign seg_out = segments[active_digit] ^ {4{bp_phase}};".into());
    m.body.push("assign backplane = bp_phase;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge clk or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    bp_phase <= 1'b0;".into());
    m.body.push("    active_digit <= 4'd0;".into());
    m.body.push("  end else begin".into());
    m.body.push("    if (cs && wr)".into());
    m.body.push("      segments[digit_sel] <= data_in;".into());
    m.body.push("    // Advance multiplex and toggle backplane".into());
    m.body.push("    active_digit <= active_digit + 4'd1;".into());
    m.body.push("    if (active_digit == 4'd15)".into());
    m.body.push("      bp_phase <= ~bp_phase;".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4702 EPROM.
pub fn chip_i4702() -> Module {
    let mut m = Module::new("i4702");
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::inout("data").width(4));
    m.ports.push(Port::input("cm_rom"));
    m.ports.push(Port::input("vpp")); // programming voltage

    m.body.push("parameter CHIP_ID = 4'd0;".into());
    m.body.push(String::new());
    m.body.push("reg [7:0] eprom [0:255];".into());
    m.body.push("reg [7:0] addr_latch;".into());
    m.body.push("reg selected;".into());
    m.body.push("reg [3:0] data_out;".into());
    m.body.push("reg data_drive;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push(String::new());
    m.body.push("assign data = data_drive ? data_out : 4'bz;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi1 or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0;".into());
    m.body.push("    selected <= 1'b0;".into());
    m.body.push("    data_drive <= 1'b0;".into());
    m.body.push("  end else begin".into());
    m.body
        .push("    phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("  end".into());
    m.body.push("end".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi2) begin".into());
    m.body.push("  case (phase)".into());
    m.body.push("    3'd0: addr_latch[3:0] <= data; // A1".into());
    m.body.push("    3'd1: addr_latch[7:4] <= data; // A2".into());
    m.body.push("    3'd2: selected <= cm_rom; // A3".into());
    m.body
        .push("    3'd3: if (selected) begin data_out <= eprom[addr_latch][3:0]; data_drive <= 1'b1; end // M1".into());
    m.body
        .push("    3'd4: if (selected) begin data_out <= eprom[addr_latch][7:4]; end // M2".into());
    m.body.push("    default: data_drive <= 1'b0;".into());
    m.body.push("  endcase".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 3205 1-of-8 decoder.
pub fn chip_i3205() -> Module {
    let mut m = Module::new("i3205");
    m.ports.push(Port::input("addr").width(3));
    m.ports.push(Port::input("e1_n"));
    m.ports.push(Port::input("e2_n"));
    m.ports.push(Port::input("e3"));
    m.ports.push(Port::output("y").width(8));

    m.body
        .push("// Active-low outputs: selected line is 0, others are 1".into());
    m.body.push("wire enabled = !e1_n && !e2_n && e3;".into());
    m.body.push("assign y = enabled ? ~(8'd1 << addr) : 8'hFF;".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 3404 latch + NAND.
pub fn chip_i3404() -> Module {
    let mut m = Module::new("i3404");
    m.ports.push(Port::input("clk"));
    m.ports.push(Port::input("d").width(6));
    m.ports.push(Port::output("q").width(6));
    m.ports.push(Port::input("nand_a1"));
    m.ports.push(Port::input("nand_a2"));
    m.ports.push(Port::output("nand_a_out"));
    m.ports.push(Port::input("nand_b1"));
    m.ports.push(Port::input("nand_b2"));
    m.ports.push(Port::output("nand_b_out"));

    m.body.push("reg [5:0] latch;".into());
    m.body.push(String::new());
    m.body.push("assign q = latch;".into());
    m.body.push("assign nand_a_out = ~(nand_a1 & nand_a2);".into());
    m.body.push("assign nand_b_out = ~(nand_b1 & nand_b2);".into());
    m.body.push(String::new());

    m.body.push("always @(posedge clk)".into());
    m.body.push("  latch <= d;".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 2101 SRAM.
pub fn chip_i2101() -> Module {
    let mut m = Module::new("i2101");
    m.ports.push(Port::input("addr").width(8));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::input("ce_n"));
    m.ports.push(Port::input("we_n"));
    m.ports.push(Port::input("oe_n"));

    m.body.push("reg [3:0] mem [0:255];".into());
    m.body.push(String::new());
    m.body.push("// Asynchronous SRAM: active-low controls".into());
    m.body
        .push("assign data_out = (!ce_n && !oe_n && we_n) ? mem[addr] : 4'bz;".into());
    m.body.push(String::new());
    m.body.push("always @(*) begin".into());
    m.body.push("  if (!ce_n && !we_n)".into());
    m.body.push("    mem[addr] = data_in;".into());
    m.body.push("end".into());

    m
}

/// Generate a synthesizable Verilog module for the Intel 4040 CPU.
pub fn chip_i4040() -> Module {
    let mut m = Module::new("i4040");
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::inout("data").width(4));
    m.ports.push(Port::output("sync"));
    m.ports.push(Port::output("cm_rom"));
    m.ports.push(Port::output("cm_ram").width(4));
    m.ports.push(Port::input("test"));
    m.ports.push(Port::input("int")); // interrupt input
    m.ports.push(Port::output("stp")); // stop acknowledge

    m.body.push("reg [11:0] pc;".into());
    m.body.push("reg [11:0] stack [0:6]; // 7-level stack".into());
    m.body.push("reg [2:0] sp;".into());
    m.body.push("reg [3:0] acc;".into());
    m.body.push("reg carry;".into());
    m.body
        .push("reg [3:0] regs [0:23]; // 24 registers (3 banks of 8 pairs)".into());
    m.body.push("reg [7:0] instruction;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push("reg [3:0] data_out;".into());
    m.body.push("reg data_drive;".into());
    m.body.push("reg int_enabled;".into());
    m.body.push("reg [1:0] reg_bank;".into());
    m.body.push(String::new());
    m.body.push("assign data = data_drive ? data_out : 4'bz;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge phi1 or posedge rst) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0;".into());
    m.body.push("    pc <= 12'd0;".into());
    m.body.push("    sp <= 3'd0;".into());
    m.body.push("    acc <= 4'd0;".into());
    m.body.push("    carry <= 1'b0;".into());
    m.body.push("    data_drive <= 1'b0;".into());
    m.body.push("    int_enabled <= 1'b0;".into());
    m.body.push("    reg_bank <= 2'd0;".into());
    m.body.push("  end else begin".into());
    m.body
        .push("    phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("  end".into());
    m.body.push("end".into());
    m.body.push(String::new());

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

/// Generate all MCS-4/MCS-40 chip modules.
pub fn all_chip_modules() -> Vec<Module> {
    vec![
        // MCS-4 core
        chip_i4004(),
        chip_i4001(),
        chip_i4002(),
        chip_i4003(),
        // MCS-4 support
        chip_i4008(),
        chip_i4009(),
        chip_i3216(),
        chip_i3226(),
        chip_i3205(),
        chip_i3404(),
        chip_i2101(),
        // MCS-40 core
        chip_i4040(),
        chip_i4101(),
        chip_i4201(),
        chip_i4289(),
        chip_i4308(),
        // MCS-40 clocks
        chip_i4207(),
        chip_i4209(),
        chip_i4211(),
        // MCS-40 peripherals
        chip_i4265(),
        chip_i4316(),
        chip_i4702(),
    ]
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
    fn all_chip_modules_returns_all() {
        let modules = all_chip_modules();
        assert_eq!(modules.len(), 22);
        let names: Vec<&str> = modules.iter().map(|m| m.name.as_str()).collect();
        // MCS-4 core
        assert!(names.contains(&"i4004"));
        assert!(names.contains(&"i4001"));
        assert!(names.contains(&"i4002"));
        assert!(names.contains(&"i4003"));
        // MCS-4 support
        assert!(names.contains(&"i4008"));
        assert!(names.contains(&"i4009"));
        assert!(names.contains(&"i3216"));
        assert!(names.contains(&"i3226"));
        assert!(names.contains(&"i3205"));
        assert!(names.contains(&"i3404"));
        assert!(names.contains(&"i2101"));
        // MCS-40
        assert!(names.contains(&"i4040"));
        assert!(names.contains(&"i4101"));
        assert!(names.contains(&"i4201"));
        assert!(names.contains(&"i4289"));
        assert!(names.contains(&"i4308"));
        assert!(names.contains(&"i4207"));
        assert!(names.contains(&"i4209"));
        assert!(names.contains(&"i4211"));
        assert!(names.contains(&"i4265"));
        assert!(names.contains(&"i4316"));
        assert!(names.contains(&"i4702"));
    }

    #[test]
    fn i4008_address_latch() {
        let m = chip_i4008();
        let v = render_module(&m);
        assert!(v.contains("module i4008"));
        assert!(v.contains("addr_latch"));
        assert!(v.contains("addr_valid"));
    }

    #[test]
    fn i4009_io_expander() {
        let m = chip_i4009();
        let v = render_module(&m);
        assert!(v.contains("module i4009"));
        assert!(v.contains("ram_bank"));
        assert!(v.contains("io_out"));
    }

    #[test]
    fn i3216_noninverting_buffer() {
        let m = chip_i3216();
        let v = render_module(&m);
        assert!(v.contains("module i3216"));
        assert!(v.contains("port_a"));
        assert!(v.contains("port_b"));
        assert!(!v.contains("~port_a"), "3216 should be non-inverting");
    }

    #[test]
    fn i3226_inverting_buffer() {
        let m = chip_i3226();
        let v = render_module(&m);
        assert!(v.contains("module i3226"));
        assert!(v.contains("~port_a"), "3226 should be inverting");
    }

    #[test]
    fn i4040_has_7_level_stack() {
        let m = chip_i4040();
        let v = render_module(&m);
        assert!(v.contains("module i4040"));
        assert!(v.contains("stack [0:6]"), "4040 has 7-level stack");
        assert!(v.contains("[2:0] sp"), "4040 needs 3-bit SP for 7 levels");
        assert!(v.contains("int_enabled"));
    }

    #[test]
    fn i4201_clock_generator() {
        let m = chip_i4201();
        let v = render_module(&m);
        assert!(v.contains("module i4201"));
        assert!(v.contains("phi1"));
        assert!(v.contains("phi2"));
        assert!(v.contains("rst_out"));
        assert!(v.contains("stp_out"));
    }

    #[test]
    fn i4289_memory_interface() {
        let m = chip_i4289();
        let v = render_module(&m);
        assert!(v.contains("module i4289"));
        assert!(v.contains("mem_addr"));
        assert!(v.contains("oe_n"));
        assert!(v.contains("we_n"));
    }

    #[test]
    fn i4308_rom_1k() {
        let m = chip_i4308();
        let v = render_module(&m);
        assert!(v.contains("module i4308"));
        assert!(v.contains("rom [0:1023]"), "4308 has 1K ROM");
        assert!(v.contains("[9:0] addr_latch"), "4308 has 10-bit address");
    }

    #[test]
    fn i3205_decoder() {
        let m = chip_i3205();
        let v = render_module(&m);
        assert!(v.contains("module i3205"));
        assert!(v.contains("8'd1 << addr"), "should decode address to one-hot");
    }

    #[test]
    fn i3404_latch_nand() {
        let m = chip_i3404();
        let v = render_module(&m);
        assert!(v.contains("module i3404"));
        assert!(v.contains("[5:0] latch"), "6-bit latch");
        assert!(v.contains("nand_a_out"));
        assert!(v.contains("nand_b_out"));
    }

    #[test]
    fn i4265_programmable_io() {
        let m = chip_i4265();
        let v = render_module(&m);
        assert!(v.contains("module i4265"));
        assert!(v.contains("port_dir"));
        assert!(v.contains("io_0"));
        assert!(v.contains("io_3"));
    }

    #[test]
    fn all_modules_render_valid_verilog() {
        let modules = all_chip_modules();
        for module in &modules {
            let v = render_module(module);
            assert!(
                v.contains(&format!("module {}", module.name)),
                "Module {} missing header",
                module.name
            );
            assert!(v.contains("endmodule"), "Module {} missing endmodule", module.name);
        }
    }
}
