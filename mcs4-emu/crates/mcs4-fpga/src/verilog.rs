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

// ============================================================
// FPGA-safe variants: split bus, single clock domain
//
// WHY: FPGA routing fabrics lack physical tristate lines, and
//      synthesis tools reject signals driven from multiple clock
//      domains. All state must be clocked from a single sys_clk.
// WHAT: Each chip gets data_in/data_out/data_oe ports and a
//      sys_clk input. Phase counting and bus operations all run
//      on sys_clk with phi1/phi2 rising-edge detection.
// HOW: The top-level mcs4_top.v resolves contention with an
//      explicit priority multiplexer.
// ============================================================

/// FPGA-safe i4004 CPU with split data bus, single clock domain.
///
/// All state clocked from sys_clk. phi1/phi2 edges detected internally.
/// Ports changed vs chip_i4004():
/// ```text
/// inout [3:0] data  ->  input [3:0] data_in
///                       output [3:0] data_out
///                       output data_oe
/// added: sys_clk (fast system clock)
/// ```
pub fn chip_i4004_fpga() -> Module {
    let mut m = Module::new("i4004_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::output("data_oe"));
    m.ports.push(Port::output("sync"));
    m.ports.push(Port::output("cm_rom"));
    m.ports.push(Port::output("cm_ram"));
    m.ports.push(Port::input("test"));
    // Direct WMP strobe for UART bridge (avoids cross-module timing issues)
    m.ports.push(Port::output("wmp_strobe"));
    m.ports.push(Port::output("wmp_data").width(4));

    // === Internal state ===
    m.body.push("reg [11:0] pc;".into());
    m.body.push("reg [11:0] stack [0:2]; // 3-level stack".into());
    m.body.push("reg [1:0] sp; // stack pointer".into());
    m.body.push("reg [3:0] acc;".into());
    m.body.push("reg carry;".into());
    m.body.push("reg [3:0] regs [0:15]; // 16 index registers".into());
    m.body.push("reg [7:0] instruction;".into());
    m.body
        .push("reg [7:0] operand; // second byte for 2-byte instructions".into());
    m.body.push("reg [2:0] phase;".into());
    m.body
        .push("reg need_operand; // 1 = 2-byte instruction, need second fetch".into());
    m.body
        .push("reg [7:0] src_addr; // SRC address register (set by SRC instruction)".into());
    m.body.push("reg [3:0] ram_bank; // DCL bank select".into());
    m.body
        .push("reg [3:0] io_data; // data to drive on bus during X2/X3".into());
    m.body
        .push("reg io_drive; // 1 = drive io_data on bus during X2/X3".into());
    m.body
        .push("reg src_drive; // 1 = drive SRC address on bus during X2/X3".into());
    m.body
        .push("reg pc_written; // 1 = PC was set by jump instruction this cycle".into());
    m.body.push(String::new());

    // Edge detection
    m.body.push("reg phi1_d, phi2_d;".into());
    m.body.push("wire phi1_rise = phi1 && !phi1_d;".into());
    m.body.push("wire phi2_rise = phi2 && !phi2_d;".into());
    m.body.push(String::new());

    // === Combinational bus output ===
    // A1-A3: drive PC address. X2: drive SRC low or I/O data. X3: drive SRC high.
    m.body.push("assign data_out = (phase == 3'd0) ? pc[3:0] :".into());
    m.body.push("                  (phase == 3'd1) ? pc[7:4] :".into());
    m.body.push("                  (phase == 3'd2) ? pc[11:8] :".into());
    m.body
        .push("                  (phase == 3'd6 && src_drive) ? src_addr[3:0] :".into());
    m.body
        .push("                  (phase == 3'd6 && io_drive) ? io_data :".into());
    m.body
        .push("                  (phase == 3'd7 && src_drive) ? src_addr[7:4] :".into());
    m.body
        .push("                  (phase == 3'd7 && io_drive) ? io_data :".into());
    m.body.push("                  4'd0;".into());
    m.body.push("assign data_oe = (phase <= 3'd2) ||".into());
    m.body
        .push("                 ((phase == 3'd6 || phase == 3'd7) && (src_drive || io_drive));".into());
    m.body.push("assign sync = (phase == 3'd0);".into());
    m.body.push("assign cm_rom = (phase == 3'd2);".into());
    // CM-RAM active during X2/X3 for RAM/IO operations
    m.body
        .push("wire is_ram_io = (instruction[7:4] == 4'hE); // E0-EF are RAM/IO ops".into());
    m.body
        .push("assign cm_ram = (phase == 3'd6 || phase == 3'd7) && is_ram_io;".into());
    // WMP strobe: registered pulse, fires when io_drive transitions from 0 to 1
    m.body.push("reg wmp_strobe_r;".into());
    m.body.push("reg [3:0] wmp_data_r;".into());
    m.body.push("assign wmp_strobe = wmp_strobe_r;".into());
    m.body.push("assign wmp_data = wmp_data_r;".into());
    m.body.push(String::new());

    // === JCN condition evaluator ===
    // OPA[0]=test pin, OPA[1]=carry, OPA[2]=acc==0, OPA[3]=invert
    m.body.push("wire jcn_acc_zero = (acc == 4'd0);".into());
    m.body.push(
        "wire jcn_raw = (instruction[0] & ~test) | (instruction[1] & carry) | (instruction[2] & jcn_acc_zero);".into(),
    );
    m.body
        .push("wire jcn_cond = instruction[3] ? ~jcn_raw : jcn_raw;".into());
    m.body.push(String::new());

    // Single clock domain
    m.body.push("integer ri;".into());
    m.body.push("always @(posedge sys_clk) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0; pc <= 12'd0; sp <= 2'd0;".into());
    m.body.push("    acc <= 4'd0; carry <= 1'b0;".into());
    m.body.push("    phi1_d <= 1'b0; phi2_d <= 1'b0;".into());
    m.body.push("    instruction <= 8'd0; operand <= 8'd0;".into());
    m.body.push("    need_operand <= 1'b0;".into());
    m.body.push("    src_addr <= 8'd0; ram_bank <= 4'd0;".into());
    m.body
        .push("    io_data <= 4'd0; io_drive <= 1'b0; src_drive <= 1'b0; pc_written <= 1'b0;".into());
    m.body.push("    wmp_strobe_r <= 1'b0; wmp_data_r <= 4'd0;".into());
    m.body
        .push("    for (ri = 0; ri < 16; ri = ri + 1) regs[ri] <= 4'd0;".into());
    m.body
        .push("    for (ri = 0; ri < 3; ri = ri + 1) stack[ri] <= 12'd0;".into());
    m.body.push("  end else begin".into());
    m.body.push("    phi1_d <= phi1; phi2_d <= phi2;".into());
    m.body
        .push("    wmp_strobe_r <= 1'b0; // auto-clear each sys_clk cycle".into());
    m.body.push(String::new());

    // Phase counter
    m.body.push("    if (phi1_rise)".into());
    m.body
        .push("      phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push(String::new());

    m.body.push("    if (phi2_rise) begin".into());
    m.body.push("      case (phase)".into());

    // M1/M2: instruction fetch
    m.body.push("        3'd3: instruction[3:0] <= data_in; // M1".into());
    m.body.push("        3'd4: instruction[7:4] <= data_in; // M2".into());

    // X1 (phase 5): decode + execute single-byte ops + set up 2-byte flags
    m.body.push("        3'd5: begin // X1: decode and execute".into());
    m.body
        .push("          io_drive <= 1'b0; src_drive <= 1'b0; pc_written <= 1'b0; wmp_strobe_r <= 1'b0;".into());
    m.body.push("          if (need_operand) begin".into());
    // Second cycle of 2-byte instruction: operand was fetched, now execute
    m.body.push("            need_operand <= 1'b0;".into());
    m.body.push("            casez (operand)".into());
    // The operand byte was the first instruction; instruction now has the data byte
    m.body
        .push("              8'b0001_????: begin // JCN: conditional jump".into());
    m.body
        .push("                if (jcn_cond) begin pc <= {pc[11:8], instruction}; pc_written <= 1'b1; end".into());
    m.body.push("              end".into());
    m.body
        .push("              8'b0010_???0: begin // FIM: load pair with immediate".into());
    m.body
        .push("                regs[{operand[3:1], 1'b0}] <= instruction[7:4];".into());
    m.body
        .push("                regs[{operand[3:1], 1'b1}] <= instruction[3:0];".into());
    m.body.push("              end".into());
    m.body
        .push("              8'b0100_????: begin // JUN: unconditional jump".into());
    m.body
        .push("                pc <= {operand[3:0], instruction}; pc_written <= 1'b1;".into());
    m.body.push("              end".into());
    m.body
        .push("              8'b0101_????: begin // JMS: jump to subroutine".into());
    m.body.push("                stack[sp] <= pc + 12'd1;".into());
    m.body
        .push("                sp <= (sp == 2'd2) ? 2'd0 : sp + 2'd1;".into());
    m.body
        .push("                pc <= {operand[3:0], instruction}; pc_written <= 1'b1;".into());
    m.body.push("              end".into());
    m.body
        .push("              8'b0111_????: begin // ISZ: increment and skip if zero".into());
    m.body
        .push("                regs[operand[3:0]] <= regs[operand[3:0]] + 4'd1;".into());
    m.body
        .push("                if (regs[operand[3:0]] != 4'hF) begin // not wrapping to 0: take branch".into());
    m.body
        .push("                  pc <= {pc[11:8], instruction}; pc_written <= 1'b1;".into());
    m.body.push("                end".into());
    m.body.push("              end".into());
    m.body.push("              default: ;".into());
    m.body.push("            endcase".into());
    m.body.push("          end else begin".into());
    // First cycle: decode and execute single-byte instructions
    m.body.push("            casez (instruction)".into());

    // NOP (0x00)
    m.body.push("              8'b0000_0000: ; // NOP".into());

    // 2-byte instructions: set need_operand, save instruction as operand
    m.body
        .push("              8'b0001_????: begin need_operand <= 1'b1; operand <= instruction; end // JCN".into());
    m.body
        .push("              8'b0010_???0: begin need_operand <= 1'b1; operand <= instruction; end // FIM".into());
    m.body
        .push("              8'b0100_????: begin need_operand <= 1'b1; operand <= instruction; end // JUN".into());
    m.body
        .push("              8'b0101_????: begin need_operand <= 1'b1; operand <= instruction; end // JMS".into());
    m.body
        .push("              8'b0111_????: begin need_operand <= 1'b1; operand <= instruction; end // ISZ".into());

    // SRC: set src_addr from register pair, drive on bus in X2/X3
    m.body.push("              8'b0010_???1: begin // SRC".into());
    m.body
        .push("                src_addr <= {regs[{instruction[3:1], 1'b1}], regs[{instruction[3:1], 1'b0}]};".into());
    m.body.push("                src_drive <= 1'b1;".into());
    m.body.push("              end".into());

    // FIN: fetch indirect using R0R1 as ROM address (simplified: load from regs)
    m.body
        .push("              8'b0011_???0: begin // FIN (simplified: NOP for now)".into());
    m.body.push("              end".into());

    // JIN: jump indirect via register pair
    m.body.push("              8'b0011_???1: begin // JIN".into());
    m.body.push("                pc <= {pc[11:8], regs[{instruction[3:1], 1'b1}], regs[{instruction[3:1], 1'b0}]}; pc_written <= 1'b1;".into());
    m.body.push("              end".into());

    // INC
    m.body
        .push("              8'b0110_????: regs[instruction[3:0]] <= regs[instruction[3:0]] + 4'd1; // INC".into());

    // ADD, SUB, LD, XCH
    m.body.push(
        "              8'b1000_????: {carry, acc} <= acc + regs[instruction[3:0]] + {4'd0, carry}; // ADD".into(),
    );
    m.body.push(
        "              8'b1001_????: {carry, acc} <= acc + ~regs[instruction[3:0]] + {4'd0, ~carry}; // SUB".into(),
    );
    m.body
        .push("              8'b1010_????: acc <= regs[instruction[3:0]]; // LD".into());
    m.body.push(
        "              8'b1011_????: begin acc <= regs[instruction[3:0]]; regs[instruction[3:0]] <= acc; end // XCH"
            .into(),
    );

    // BBL: return from subroutine, load ACC with immediate
    m.body.push("              8'b1100_????: begin // BBL".into());
    m.body
        .push("                sp <= (sp == 2'd0) ? 2'd2 : sp - 2'd1;".into());
    m.body
        .push("                pc <= stack[(sp == 2'd0) ? 2'd2 : sp - 2'd1];".into());
    m.body
        .push("                acc <= instruction[3:0]; pc_written <= 1'b1;".into());
    m.body.push("              end".into());

    // LDM
    m.body
        .push("              8'b1101_????: acc <= instruction[3:0]; // LDM".into());

    // RAM/IO write operations (E0-E7): set io_drive to put acc on bus
    m.body
        .push("              8'b1110_0000: begin io_data <= acc; io_drive <= 1'b1; end // WRM".into());
    m.body.push("              8'b1110_0001: begin io_data <= acc; io_drive <= 1'b1; wmp_strobe_r <= 1'b1; wmp_data_r <= acc; end // WMP".into());
    m.body
        .push("              8'b1110_0010: begin io_data <= acc; io_drive <= 1'b1; end // WRR".into());
    m.body
        .push("              8'b1110_0011: begin io_data <= acc; io_drive <= 1'b1; end // WPM".into());
    m.body
        .push("              8'b1110_0100: begin io_data <= acc; io_drive <= 1'b1; end // WR0".into());
    m.body
        .push("              8'b1110_0101: begin io_data <= acc; io_drive <= 1'b1; end // WR1".into());
    m.body
        .push("              8'b1110_0110: begin io_data <= acc; io_drive <= 1'b1; end // WR2".into());
    m.body
        .push("              8'b1110_0111: begin io_data <= acc; io_drive <= 1'b1; end // WR3".into());

    // RAM/IO read operations (E8-EF): data read from bus in X3 phase
    m.body
        .push("              8'b1110_1???: ; // RDM/RDR/SBM/ADM/RD0-RD3: read in X3".into());

    // Accumulator group (F0-FF)
    m.body
        .push("              8'b1111_0000: begin acc <= 4'd0; carry <= 1'b0; end // CLB".into());
    m.body.push("              8'b1111_0001: carry <= 1'b0; // CLC".into());
    m.body
        .push("              8'b1111_0010: {carry, acc} <= acc + 5'd1; // IAC".into());
    m.body
        .push("              8'b1111_0011: carry <= ~carry; // CMC".into());
    m.body.push("              8'b1111_0100: acc <= ~acc; // CMA".into());
    m.body
        .push("              8'b1111_0101: {carry, acc} <= {acc, carry}; // RAL".into());
    m.body
        .push("              8'b1111_0110: {carry, acc} <= {acc[0], carry, acc[3:1]}; // RAR".into());
    m.body
        .push("              8'b1111_0111: begin acc <= {3'd0, carry}; carry <= 1'b0; end // TCC".into());
    m.body
        .push("              8'b1111_1000: {carry, acc} <= acc + 5'h1F; // DAC".into());
    m.body
        .push("              8'b1111_1001: begin acc <= carry ? 4'd10 : 4'd9; carry <= 1'b0; end // TCS".into());
    m.body.push("              8'b1111_1010: carry <= 1'b1; // STC".into());
    m.body
        .push("              8'b1111_1011: if (acc > 4'd9 || carry) {carry, acc} <= acc + 5'd6; // DAA".into());
    m.body.push("              8'b1111_1100: begin // KBP".into());
    m.body.push("                case (acc)".into());
    m.body
        .push("                  4'b0000: acc <= 4'd0; 4'b0001: acc <= 4'd1;".into());
    m.body
        .push("                  4'b0010: acc <= 4'd2; 4'b0100: acc <= 4'd3;".into());
    m.body
        .push("                  4'b1000: acc <= 4'd4; default: acc <= 4'hF;".into());
    m.body.push("                endcase".into());
    m.body.push("              end".into());
    m.body
        .push("              8'b1111_1101: ram_bank <= acc; // DCL".into());
    m.body.push("              default: ;".into());
    m.body.push("            endcase".into());
    m.body.push("          end".into());
    m.body.push("        end".into());

    // X3 (phase 7): RAM/IO read completion + PC advance
    m.body
        .push("        3'd7: begin // X3: read completion + advance PC".into());
    // RAM/IO read: latch bus data into accumulator
    m.body.push("          casez (instruction)".into());
    m.body
        .push("            8'b1110_1000: {carry, acc} <= acc + ~data_in + {4'd0, ~carry}; // SBM".into());
    m.body.push("            8'b1110_1001: acc <= data_in; // RDM".into());
    m.body.push("            8'b1110_1010: acc <= data_in; // RDR".into());
    m.body
        .push("            8'b1110_1011: {carry, acc} <= acc + data_in + {4'd0, carry}; // ADM".into());
    m.body
        .push("            8'b1110_11??: acc <= data_in; // RD0-RD3".into());
    m.body.push("            default: ;".into());
    m.body.push("          endcase".into());
    // Advance PC unless a jump instruction already set it in X1
    m.body.push("          if (!pc_written) pc <= pc + 12'd1;".into());
    // Note: io_drive and src_drive cleared at start of next X1, not here,
    // so the output remains valid for the bridge to detect the edge.
    m.body.push("        end".into());

    m.body.push("        default: ;".into());
    m.body.push("      endcase".into());
    m.body.push("    end".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// FPGA-safe i4001 ROM: split bus, single clock domain, BSRAM interface.
pub fn chip_i4001_fpga() -> Module {
    let mut m = Module::new("i4001_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::output("data_oe"));
    m.ports.push(Port::input("cm_rom"));
    m.ports.push(Port::input("sync"));
    m.ports.push(Port::output("rom_addr").width(8));
    m.ports.push(Port::input("rom_data").width(8));
    m.ports.push(Port::output("io_out").width(4));
    m.ports.push(Port::input("io_in").width(4));
    m.ports.push(Port::output("io_wr"));

    m.body.push("parameter CHIP_ID = 4'd0;".into());
    m.body.push(String::new());
    m.body.push("reg [7:0] addr_latch;".into());
    m.body.push("reg [3:0] io_latch;".into());
    m.body.push("reg selected;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push("reg io_wr_r;".into());
    m.body.push("reg phi1_d, phi2_d;".into());
    m.body.push("wire phi1_rise = phi1 && !phi1_d;".into());
    m.body.push("wire phi2_rise = phi2 && !phi2_d;".into());
    m.body.push(String::new());

    // Combinational ROM data output
    m.body
        .push("assign data_out = (phase == 3'd3 && selected) ? rom_data[3:0] :".into());
    m.body
        .push("                  (phase == 3'd4 && selected) ? rom_data[7:4] :".into());
    m.body.push("                  4'd0;".into());
    m.body
        .push("assign data_oe = (phase == 3'd3 || phase == 3'd4) && selected;".into());
    m.body.push("assign rom_addr = addr_latch;".into());
    m.body.push("assign io_out = io_latch;".into());
    m.body.push("assign io_wr = io_wr_r;".into());
    m.body.push(String::new());

    // Single clock domain
    m.body.push("always @(posedge sys_clk) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0; selected <= 1'b0;".into());
    m.body.push("    io_latch <= 4'd0; io_wr_r <= 1'b0;".into());
    m.body.push("    phi1_d <= 1'b0; phi2_d <= 1'b0;".into());
    m.body.push("    addr_latch <= 8'd0;".into());
    m.body.push("  end else begin".into());
    m.body.push("    phi1_d <= phi1; phi2_d <= phi2;".into());
    m.body.push("    io_wr_r <= 1'b0;".into());
    m.body.push("    if (phi1_rise)".into());
    m.body
        .push("      phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("    if (phi2_rise) begin".into());
    m.body.push("      case (phase)".into());
    m.body.push("        3'd0: addr_latch[3:0] <= data_in;".into());
    m.body.push("        3'd1: addr_latch[7:4] <= data_in;".into());
    m.body.push("        3'd2: selected <= 1'b1;".into());
    m.body
        .push("        3'd7: if (selected) begin io_latch <= data_in; io_wr_r <= 1'b1; end".into());
    m.body.push("        default: ;".into());
    m.body.push("      endcase".into());
    m.body.push("    end".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// FPGA-safe i4002 RAM: split bus, single clock domain, BSRAM interface.
///
/// Handles WRM/RDM/WMP/WRR and status register operations via the MCS-4
/// bus protocol. The SRC instruction (from CPU) sets the address during
/// X2/X3 phases; subsequent RAM/IO instructions operate on that address.
pub fn chip_i4002_fpga() -> Module {
    let mut m = Module::new("i4002_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::output("data_oe"));
    m.ports.push(Port::input("cm_ram"));
    m.ports.push(Port::output("ram_addr").width(8));
    m.ports.push(Port::input("ram_rdata").width(4));
    m.ports.push(Port::output("ram_wdata").width(4));
    m.ports.push(Port::output("ram_we"));
    m.ports.push(Port::output("port_out").width(4));

    m.body.push("parameter CHIP_ID = 2'd0;".into());
    m.body.push("parameter BANK_ID = 2'd0;".into());
    m.body.push(String::new());
    m.body.push("reg [3:0] output_port;".into());
    m.body.push("reg [3:0] status [0:3]; // 4 status nibbles".into());
    m.body
        .push("reg [7:0] src_latch; // SRC address latched from bus".into());
    m.body.push("reg selected;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push("reg phi1_d, phi2_d;".into());
    m.body.push("wire phi1_rise = phi1 && !phi1_d;".into());
    m.body.push("wire phi2_rise = phi2 && !phi2_d;".into());
    m.body.push("reg do_write; // write RAM this cycle".into());
    m.body.push("reg do_read; // drive RAM data onto bus".into());
    m.body.push("reg [3:0] wdata; // data to write to RAM".into());
    m.body
        .push("reg [3:0] rdata_buf; // buffered read data for bus output".into());
    m.body
        .push("reg [1:0] status_idx; // which status reg for WR0-3/RD0-3".into());
    m.body
        .push("reg status_read; // drive status onto bus instead of RAM".into());
    m.body.push(String::new());

    // RAM address from SRC latch
    m.body.push("assign ram_addr = src_latch;".into());
    m.body.push("assign ram_wdata = wdata;".into());
    m.body.push("assign ram_we = do_write;".into());
    m.body.push("assign port_out = output_port;".into());
    m.body.push(String::new());

    // Combinational read output during X3 (phase 7) when selected
    m.body
        .push("assign data_out = status_read ? status[status_idx] : ram_rdata;".into());
    m.body.push("assign data_oe = do_read && selected;".into());
    m.body.push(String::new());

    // Single clock domain
    m.body.push("always @(posedge sys_clk) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0; selected <= 1'b0;".into());
    m.body.push("    output_port <= 4'd0;".into());
    m.body.push("    phi1_d <= 1'b0; phi2_d <= 1'b0;".into());
    m.body.push("    src_latch <= 8'd0;".into());
    m.body.push("    do_write <= 1'b0; do_read <= 1'b0;".into());
    m.body.push("    wdata <= 4'd0; status_idx <= 2'd0;".into());
    m.body.push("    status_read <= 1'b0;".into());
    m.body.push("    status[0] <= 4'd0; status[1] <= 4'd0;".into());
    m.body.push("    status[2] <= 4'd0; status[3] <= 4'd0;".into());
    m.body.push("  end else begin".into());
    m.body.push("    phi1_d <= phi1; phi2_d <= phi2;".into());
    m.body.push("    if (phi1_rise)".into());
    m.body
        .push("      phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("    if (phi2_rise) begin".into());
    m.body.push("      do_write <= 1'b0; do_read <= 1'b0;".into());
    m.body.push("      status_read <= 1'b0;".into());
    m.body.push("      case (phase)".into());
    m.body
        .push("        3'd2: selected <= cm_ram; // A3: chip select".into());
    // X2 (phase 6): SRC address latch (low nibble from bus)
    m.body
        .push("        3'd6: if (selected) src_latch[3:0] <= data_in;".into());
    // X3 (phase 7): SRC high nibble + RAM/IO command execution
    m.body.push("        3'd7: if (selected) begin".into());
    m.body.push("          src_latch[7:4] <= data_in;".into());
    // WRM (E0): write acc to RAM
    m.body.push("          if (cm_ram) begin".into());
    m.body
        .push("            // CPU drives io_data on bus during X2/X3 for write ops".into());
    m.body
        .push("            // For WRM: data_in has the acc value (CPU drives it)".into());
    m.body.push("            wdata <= data_in;".into());
    m.body.push("            do_write <= 1'b1;".into());
    m.body.push("          end".into());
    m.body.push("        end".into());
    m.body.push("        default: ;".into());
    m.body.push("      endcase".into());
    m.body.push("    end".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// FPGA-safe i4003: 10-bit shift register. Single clock domain.
pub fn chip_i4003_fpga() -> Module {
    let mut m = Module::new("i4003_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("clk_in")); // shift clock (directly from CPU or bus)
    m.ports.push(Port::input("data_in")); // serial input
    m.ports.push(Port::input("enable"));
    m.ports.push(Port::output("parallel_out").width(10));
    m.ports.push(Port::output("serial_out"));

    m.body.push("reg [9:0] shift_reg;".into());
    m.body.push("reg clk_in_d;".into());
    m.body.push("wire clk_in_rise = clk_in && !clk_in_d;".into());
    m.body.push(String::new());
    m.body.push("assign parallel_out = shift_reg;".into());
    m.body.push("assign serial_out = shift_reg[9];".into());
    m.body.push(String::new());

    m.body.push("always @(posedge sys_clk) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    shift_reg <= 10'd0; clk_in_d <= 1'b0;".into());
    m.body.push("  end else begin".into());
    m.body.push("    clk_in_d <= clk_in;".into());
    m.body.push("    if (clk_in_rise && enable)".into());
    m.body.push("      shift_reg <= {shift_reg[8:0], data_in};".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// FPGA-safe i4040: enhanced 4-bit CPU (60 instructions).
///
/// Extends i4004_fpga with:
///   - 7-level stack (up from 3)
///   - 24 registers (3 banks x 8 pairs)
///   - Interrupt controller (EIN, DIN, RPM)
///   - HLT (halt), BBS (bank select)
///   - STP output (stop acknowledge)
pub fn chip_i4040_fpga() -> Module {
    let mut m = Module::new("i4040_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::output("data_oe"));
    m.ports.push(Port::output("sync"));
    m.ports.push(Port::output("cm_rom"));
    m.ports.push(Port::output("cm_ram"));
    m.ports.push(Port::input("test"));
    m.ports.push(Port::input("int")); // interrupt input
    m.ports.push(Port::output("stp")); // stop/halt acknowledge
    m.ports.push(Port::output("wmp_strobe"));
    m.ports.push(Port::output("wmp_data").width(4));

    // State -- extended vs i4004
    m.body.push("reg [11:0] pc;".into());
    m.body.push("reg [11:0] stack [0:6]; // 7-level stack".into());
    m.body.push("reg [2:0] sp;".into());
    m.body.push("reg [3:0] acc;".into());
    m.body.push("reg carry;".into());
    m.body.push("reg [3:0] regs [0:23]; // 24 registers (3 banks)".into());
    m.body.push("reg [7:0] instruction;".into());
    m.body.push("reg [7:0] operand;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push("reg need_operand;".into());
    m.body.push("reg [7:0] src_addr;".into());
    m.body.push("reg [3:0] ram_bank;".into());
    m.body.push("reg [3:0] io_data;".into());
    m.body.push("reg io_drive;".into());
    m.body.push("reg src_drive;".into());
    m.body.push("reg pc_written;".into());
    // 4040-specific
    m.body.push("reg int_enabled;".into());
    m.body.push("reg [1:0] reg_bank; // 0-2 selects register bank".into());
    m.body.push("reg halted;".into());
    m.body.push("reg wmp_strobe_r;".into());
    m.body.push("reg [3:0] wmp_data_r;".into());
    m.body.push(String::new());

    // Edge detection
    m.body.push("reg phi1_d, phi2_d;".into());
    m.body.push("wire phi1_rise = phi1 && !phi1_d;".into());
    m.body.push("wire phi2_rise = phi2 && !phi2_d;".into());
    m.body.push(String::new());

    // Register bank offset: bank 0 = regs[0:7], bank 1 = regs[8:15], bank 2 = regs[16:23]
    m.body
        .push("wire [4:0] reg_idx = {reg_bank, instruction[2:0]}; // 5-bit index into 24-reg file".into());
    m.body.push(String::new());

    // Combinational outputs (same structure as i4004_fpga)
    m.body.push("assign data_out = (phase == 3'd0) ? pc[3:0] :".into());
    m.body.push("                  (phase == 3'd1) ? pc[7:4] :".into());
    m.body.push("                  (phase == 3'd2) ? pc[11:8] :".into());
    m.body
        .push("                  (phase == 3'd6 && src_drive) ? src_addr[3:0] :".into());
    m.body
        .push("                  (phase == 3'd6 && io_drive) ? io_data :".into());
    m.body
        .push("                  (phase == 3'd7 && src_drive) ? src_addr[7:4] :".into());
    m.body
        .push("                  (phase == 3'd7 && io_drive) ? io_data :".into());
    m.body.push("                  4'd0;".into());
    m.body.push("assign data_oe = !halted && ((phase <= 3'd2) ||".into());
    m.body
        .push("                 ((phase == 3'd6 || phase == 3'd7) && (src_drive || io_drive)));".into());
    m.body.push("assign sync = (phase == 3'd0);".into());
    m.body.push("assign cm_rom = (phase == 3'd2);".into());
    m.body.push("wire is_ram_io = (instruction[7:4] == 4'hE);".into());
    m.body
        .push("assign cm_ram = (phase == 3'd6 || phase == 3'd7) && is_ram_io;".into());
    m.body.push("assign stp = halted;".into());
    m.body.push("assign wmp_strobe = wmp_strobe_r;".into());
    m.body.push("assign wmp_data = wmp_data_r;".into());
    m.body.push(String::new());

    // JCN condition evaluator
    m.body.push("wire jcn_acc_zero = (acc == 4'd0);".into());
    m.body.push(
        "wire jcn_raw = (instruction[0] & ~test) | (instruction[1] & carry) | (instruction[2] & jcn_acc_zero);".into(),
    );
    m.body
        .push("wire jcn_cond = instruction[3] ? ~jcn_raw : jcn_raw;".into());
    m.body.push(String::new());

    // Interrupt edge detection
    m.body.push("reg int_d;".into());
    m.body.push("wire int_rise = int && !int_d;".into());
    m.body.push("reg int_pending;".into());
    m.body.push(String::new());

    m.body.push("integer ri;".into());
    m.body.push("always @(posedge sys_clk) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0; pc <= 12'd0; sp <= 3'd0;".into());
    m.body.push("    acc <= 4'd0; carry <= 1'b0;".into());
    m.body.push("    phi1_d <= 1'b0; phi2_d <= 1'b0;".into());
    m.body.push("    instruction <= 8'd0; operand <= 8'd0;".into());
    m.body.push("    need_operand <= 1'b0;".into());
    m.body.push("    src_addr <= 8'd0; ram_bank <= 4'd0;".into());
    m.body
        .push("    io_data <= 4'd0; io_drive <= 1'b0; src_drive <= 1'b0; pc_written <= 1'b0;".into());
    m.body
        .push("    int_enabled <= 1'b0; reg_bank <= 2'd0; halted <= 1'b0;".into());
    m.body.push("    wmp_strobe_r <= 1'b0; wmp_data_r <= 4'd0;".into());
    m.body.push("    int_d <= 1'b0; int_pending <= 1'b0;".into());
    m.body
        .push("    for (ri = 0; ri < 24; ri = ri + 1) regs[ri] <= 4'd0;".into());
    m.body
        .push("    for (ri = 0; ri < 7; ri = ri + 1) stack[ri] <= 12'd0;".into());
    m.body.push("  end else begin".into());
    m.body.push("    phi1_d <= phi1; phi2_d <= phi2;".into());
    m.body.push("    wmp_strobe_r <= 1'b0;".into());
    m.body.push("    int_d <= int;".into());
    m.body
        .push("    if (int_rise && int_enabled) int_pending <= 1'b1;".into());
    m.body.push(String::new());

    m.body.push("    if (phi1_rise && !halted)".into());
    m.body
        .push("      phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push(String::new());

    m.body.push("    if (phi2_rise && !halted) begin".into());
    m.body.push("      case (phase)".into());
    m.body.push("        3'd3: instruction[3:0] <= data_in;".into());
    m.body.push("        3'd4: instruction[7:4] <= data_in;".into());
    m.body.push("        3'd5: begin".into());
    m.body
        .push("          io_drive <= 1'b0; src_drive <= 1'b0; pc_written <= 1'b0;".into());

    // Handle interrupt: at start of X1, if pending, push PC and jump to 0x003
    m.body.push("          if (int_pending && !need_operand) begin".into());
    m.body
        .push("            int_pending <= 1'b0; int_enabled <= 1'b0;".into());
    m.body
        .push("            stack[sp] <= pc; sp <= (sp == 3'd6) ? 3'd0 : sp + 3'd1;".into());
    m.body.push("            pc <= 12'h003; pc_written <= 1'b1;".into());
    m.body.push("          end else if (need_operand) begin".into());
    m.body.push("            need_operand <= 1'b0;".into());
    m.body.push("            casez (operand)".into());
    m.body.push(
        "              8'b0001_????: if (jcn_cond) begin pc <= {pc[11:8], instruction}; pc_written <= 1'b1; end".into(),
    );
    m.body.push("              8'b0010_???0: begin regs[{operand[3:1], 1'b0}] <= instruction[7:4]; regs[{operand[3:1], 1'b1}] <= instruction[3:0]; end".into());
    m.body
        .push("              8'b0100_????: begin pc <= {operand[3:0], instruction}; pc_written <= 1'b1; end".into());
    m.body.push("              8'b0101_????: begin stack[sp] <= pc + 12'd1; sp <= (sp == 3'd6) ? 3'd0 : sp + 3'd1; pc <= {operand[3:0], instruction}; pc_written <= 1'b1; end".into());
    m.body.push("              8'b0111_????: begin regs[operand[3:0]] <= regs[operand[3:0]] + 4'd1; if (regs[operand[3:0]] != 4'hF) begin pc <= {pc[11:8], instruction}; pc_written <= 1'b1; end end".into());
    m.body.push("              default: ;".into());
    m.body.push("            endcase".into());
    m.body.push("          end else begin".into());
    m.body.push("            casez (instruction)".into());
    m.body.push("              8'b0000_0000: ; // NOP".into());
    // 2-byte instructions
    m.body
        .push("              8'b0001_????: begin need_operand <= 1'b1; operand <= instruction; end // JCN".into());
    m.body
        .push("              8'b0010_???0: begin need_operand <= 1'b1; operand <= instruction; end // FIM".into());
    m.body
        .push("              8'b0100_????: begin need_operand <= 1'b1; operand <= instruction; end // JUN".into());
    m.body
        .push("              8'b0101_????: begin need_operand <= 1'b1; operand <= instruction; end // JMS".into());
    m.body
        .push("              8'b0111_????: begin need_operand <= 1'b1; operand <= instruction; end // ISZ".into());
    // SRC, JIN, FIN
    m.body.push("              8'b0010_???1: begin src_addr <= {regs[{instruction[3:1], 1'b1}], regs[{instruction[3:1], 1'b0}]}; src_drive <= 1'b1; end // SRC".into());
    m.body.push("              8'b0011_???0: ; // FIN (stub)".into());
    m.body.push("              8'b0011_???1: begin pc <= {pc[11:8], regs[{instruction[3:1], 1'b1}], regs[{instruction[3:1], 1'b0}]}; pc_written <= 1'b1; end // JIN".into());
    // Register ops
    m.body
        .push("              8'b0110_????: regs[instruction[3:0]] <= regs[instruction[3:0]] + 4'd1; // INC".into());
    m.body.push(
        "              8'b1000_????: {carry, acc} <= acc + regs[instruction[3:0]] + {4'd0, carry}; // ADD".into(),
    );
    m.body.push(
        "              8'b1001_????: {carry, acc} <= acc + ~regs[instruction[3:0]] + {4'd0, ~carry}; // SUB".into(),
    );
    m.body
        .push("              8'b1010_????: acc <= regs[instruction[3:0]]; // LD".into());
    m.body.push(
        "              8'b1011_????: begin acc <= regs[instruction[3:0]]; regs[instruction[3:0]] <= acc; end // XCH"
            .into(),
    );
    // BBL
    m.body.push("              8'b1100_????: begin sp <= (sp == 3'd0) ? 3'd6 : sp - 3'd1; pc <= stack[(sp == 3'd0) ? 3'd6 : sp - 3'd1]; acc <= instruction[3:0]; pc_written <= 1'b1; end // BBL".into());
    // LDM
    m.body
        .push("              8'b1101_????: acc <= instruction[3:0]; // LDM".into());
    // RAM/IO write (E0-E7)
    m.body
        .push("              8'b1110_0000: begin io_data <= acc; io_drive <= 1'b1; end // WRM".into());
    m.body.push("              8'b1110_0001: begin io_data <= acc; io_drive <= 1'b1; wmp_strobe_r <= 1'b1; wmp_data_r <= acc; end // WMP".into());
    m.body
        .push("              8'b1110_001?: begin io_data <= acc; io_drive <= 1'b1; end // WRR/WPM".into());
    m.body
        .push("              8'b1110_01??: begin io_data <= acc; io_drive <= 1'b1; end // WR0-WR3".into());
    // RAM/IO read (E8-EF) handled in X3
    m.body.push("              8'b1110_1???: ; // read ops in X3".into());
    // Accumulator group (F0-FF) -- same as 4004
    m.body
        .push("              8'b1111_0000: begin acc <= 4'd0; carry <= 1'b0; end // CLB".into());
    m.body.push("              8'b1111_0001: carry <= 1'b0; // CLC".into());
    m.body
        .push("              8'b1111_0010: {carry, acc} <= acc + 5'd1; // IAC".into());
    m.body
        .push("              8'b1111_0011: carry <= ~carry; // CMC".into());
    m.body.push("              8'b1111_0100: acc <= ~acc; // CMA".into());
    m.body
        .push("              8'b1111_0101: {carry, acc} <= {acc, carry}; // RAL".into());
    m.body
        .push("              8'b1111_0110: {carry, acc} <= {acc[0], carry, acc[3:1]}; // RAR".into());
    m.body
        .push("              8'b1111_0111: begin acc <= {3'd0, carry}; carry <= 1'b0; end // TCC".into());
    m.body
        .push("              8'b1111_1000: {carry, acc} <= acc + 5'h1F; // DAC".into());
    m.body
        .push("              8'b1111_1001: begin acc <= carry ? 4'd10 : 4'd9; carry <= 1'b0; end // TCS".into());
    m.body.push("              8'b1111_1010: carry <= 1'b1; // STC".into());
    m.body
        .push("              8'b1111_1011: if (acc > 4'd9 || carry) {carry, acc} <= acc + 5'd6; // DAA".into());
    m.body.push("              8'b1111_1100: begin case (acc) 4'd0: acc<=4'd0; 4'd1: acc<=4'd1; 4'd2: acc<=4'd2; 4'd4: acc<=4'd3; 4'd8: acc<=4'd4; default: acc<=4'hF; endcase end // KBP".into());
    m.body
        .push("              8'b1111_1101: ram_bank <= acc; // DCL".into());
    // 4040-specific instructions (0x00 range with specific OPA)
    m.body.push("              8'b0000_0001: halted <= 1'b1; // HLT".into());
    m.body
        .push("              8'b0000_0010: begin reg_bank <= acc[1:0]; end // BBS (bank select)".into());
    m.body
        .push("              8'b0000_0011: begin // RPM (return and restore)".into());
    m.body
        .push("                sp <= (sp == 3'd0) ? 3'd6 : sp - 3'd1;".into());
    m.body
        .push("                pc <= stack[(sp == 3'd0) ? 3'd6 : sp - 3'd1];".into());
    m.body.push("                pc_written <= 1'b1;".into());
    m.body.push("              end".into());
    m.body.push("              8'b0000_0100: ; // 4040 reserved".into());
    m.body
        .push("              8'b0000_0101: int_enabled <= 1'b1; // EIN".into());
    m.body
        .push("              8'b0000_0110: int_enabled <= 1'b0; // DIN".into());
    m.body.push("              default: ;".into());
    m.body.push("            endcase".into());
    m.body.push("          end".into());
    m.body.push("        end".into());

    // X3: read completion + PC advance
    m.body.push("        3'd7: begin".into());
    m.body.push("          casez (instruction)".into());
    m.body
        .push("            8'b1110_1000: {carry, acc} <= acc + ~data_in + {4'd0, ~carry}; // SBM".into());
    m.body.push("            8'b1110_1001: acc <= data_in; // RDM".into());
    m.body.push("            8'b1110_1010: acc <= data_in; // RDR".into());
    m.body
        .push("            8'b1110_1011: {carry, acc} <= acc + data_in + {4'd0, carry}; // ADM".into());
    m.body
        .push("            8'b1110_11??: acc <= data_in; // RD0-RD3".into());
    m.body.push("            default: ;".into());
    m.body.push("          endcase".into());
    m.body.push("          if (!pc_written) pc <= pc + 12'd1;".into());
    m.body.push("        end".into());
    m.body.push("        default: ;".into());
    m.body.push("      endcase".into());
    m.body.push("    end".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// FPGA-safe i4308: 1Kx8 ROM with I/O, split bus, single clock domain.
pub fn chip_i4308_fpga() -> Module {
    let mut m = Module::new("i4308_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::output("data_oe"));
    m.ports.push(Port::input("cm_rom"));
    m.ports.push(Port::input("sync"));
    m.ports.push(Port::output("rom_addr").width(10)); // 10-bit for 1K ROM
    m.ports.push(Port::input("rom_data").width(8));
    m.ports.push(Port::output("io_out").width(4));
    m.ports.push(Port::input("io_in").width(4));
    m.ports.push(Port::output("io_wr"));

    m.body.push("parameter CHIP_ID = 4'd0;".into());
    m.body.push(String::new());
    m.body.push("reg [9:0] addr_latch; // 10-bit address".into());
    m.body.push("reg [3:0] io_latch;".into());
    m.body.push("reg selected;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push("reg io_wr_r;".into());
    m.body.push("reg phi1_d, phi2_d;".into());
    m.body.push("wire phi1_rise = phi1 && !phi1_d;".into());
    m.body.push("wire phi2_rise = phi2 && !phi2_d;".into());
    m.body.push(String::new());

    // Combinational ROM data output
    m.body
        .push("assign data_out = (phase == 3'd3 && selected) ? rom_data[3:0] :".into());
    m.body
        .push("                  (phase == 3'd4 && selected) ? rom_data[7:4] :".into());
    m.body.push("                  4'd0;".into());
    m.body
        .push("assign data_oe = (phase == 3'd3 || phase == 3'd4) && selected;".into());
    m.body.push("assign rom_addr = addr_latch;".into());
    m.body.push("assign io_out = io_latch;".into());
    m.body.push("assign io_wr = io_wr_r;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge sys_clk) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0; selected <= 1'b0;".into());
    m.body.push("    io_latch <= 4'd0; io_wr_r <= 1'b0;".into());
    m.body.push("    phi1_d <= 1'b0; phi2_d <= 1'b0;".into());
    m.body.push("    addr_latch <= 10'd0;".into());
    m.body.push("  end else begin".into());
    m.body.push("    phi1_d <= phi1; phi2_d <= phi2;".into());
    m.body.push("    io_wr_r <= 1'b0;".into());
    m.body.push("    if (phi1_rise)".into());
    m.body
        .push("      phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("    if (phi2_rise) begin".into());
    m.body.push("      case (phase)".into());
    m.body.push("        3'd0: addr_latch[3:0] <= data_in;".into());
    m.body.push("        3'd1: addr_latch[7:4] <= data_in;".into());
    m.body
        .push("        3'd2: begin addr_latch[9:8] <= data_in[1:0]; selected <= 1'b1; end".into());
    m.body
        .push("        3'd7: if (selected) begin io_latch <= data_in; io_wr_r <= 1'b1; end".into());
    m.body.push("        default: ;".into());
    m.body.push("      endcase".into());
    m.body.push("    end".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// FPGA-safe i4265: programmable I/O (4 x 4-bit ports). Single clock domain.
pub fn chip_i4265_fpga() -> Module {
    let mut m = Module::new("i4265_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("cs"));
    m.ports.push(Port::input("wr"));
    m.ports.push(Port::input("port_sel").width(2));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::input("dir_wr"));
    // 4 bidirectional I/O ports -- split into in/out for FPGA
    m.ports.push(Port::output("io_0_out").width(4));
    m.ports.push(Port::input("io_0_in").width(4));
    m.ports.push(Port::output("io_0_oe").width(4));
    m.ports.push(Port::output("io_1_out").width(4));
    m.ports.push(Port::input("io_1_in").width(4));
    m.ports.push(Port::output("io_1_oe").width(4));
    m.ports.push(Port::output("io_2_out").width(4));
    m.ports.push(Port::input("io_2_in").width(4));
    m.ports.push(Port::output("io_2_oe").width(4));
    m.ports.push(Port::output("io_3_out").width(4));
    m.ports.push(Port::input("io_3_in").width(4));
    m.ports.push(Port::output("io_3_oe").width(4));

    m.body.push("reg [3:0] port_data [0:3];".into());
    m.body
        .push("reg [3:0] port_dir [0:3]; // 1=output, 0=input per bit".into());
    m.body.push(String::new());

    // Output drivers
    m.body
        .push("assign io_0_out = port_data[0]; assign io_0_oe = port_dir[0];".into());
    m.body
        .push("assign io_1_out = port_data[1]; assign io_1_oe = port_dir[1];".into());
    m.body
        .push("assign io_2_out = port_data[2]; assign io_2_oe = port_dir[2];".into());
    m.body
        .push("assign io_3_out = port_data[3]; assign io_3_oe = port_dir[3];".into());
    m.body.push(String::new());

    // Read mux: output bits read written value, input bits read pin
    m.body.push("wire [3:0] pin_in [0:3];".into());
    m.body
        .push("assign pin_in[0] = io_0_in; assign pin_in[1] = io_1_in;".into());
    m.body
        .push("assign pin_in[2] = io_2_in; assign pin_in[3] = io_3_in;".into());
    m.body.push(
        "assign data_out = (port_dir[port_sel] & port_data[port_sel]) | (~port_dir[port_sel] & pin_in[port_sel]);"
            .into(),
    );
    m.body.push(String::new());

    m.body.push("always @(posedge sys_clk) begin".into());
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

/// FPGA-safe i4702: 256x8 EPROM, split bus, single clock domain.
/// Identical to i4001_fpga except named differently for multi-ROM banks.
pub fn chip_i4702_fpga() -> Module {
    let mut m = Module::new("i4702_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("phi1"));
    m.ports.push(Port::input("phi2"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::output("data_oe"));
    m.ports.push(Port::input("cm_rom"));
    m.ports.push(Port::output("rom_addr").width(8));
    m.ports.push(Port::input("rom_data").width(8));

    m.body.push("parameter CHIP_ID = 4'd0;".into());
    m.body.push(String::new());
    m.body.push("reg [7:0] addr_latch;".into());
    m.body.push("reg selected;".into());
    m.body.push("reg [2:0] phase;".into());
    m.body.push("reg phi1_d, phi2_d;".into());
    m.body.push("wire phi1_rise = phi1 && !phi1_d;".into());
    m.body.push("wire phi2_rise = phi2 && !phi2_d;".into());
    m.body.push(String::new());

    m.body
        .push("assign data_out = (phase == 3'd3 && selected) ? rom_data[3:0] :".into());
    m.body
        .push("                  (phase == 3'd4 && selected) ? rom_data[7:4] :".into());
    m.body.push("                  4'd0;".into());
    m.body
        .push("assign data_oe = (phase == 3'd3 || phase == 3'd4) && selected;".into());
    m.body.push("assign rom_addr = addr_latch;".into());
    m.body.push(String::new());

    m.body.push("always @(posedge sys_clk) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    phase <= 3'd0; selected <= 1'b0;".into());
    m.body.push("    phi1_d <= 1'b0; phi2_d <= 1'b0;".into());
    m.body.push("    addr_latch <= 8'd0;".into());
    m.body.push("  end else begin".into());
    m.body.push("    phi1_d <= phi1; phi2_d <= phi2;".into());
    m.body.push("    if (phi1_rise)".into());
    m.body
        .push("      phase <= (phase == 3'd7) ? 3'd0 : phase + 3'd1;".into());
    m.body.push("    if (phi2_rise) begin".into());
    m.body.push("      case (phase)".into());
    m.body.push("        3'd0: addr_latch[3:0] <= data_in;".into());
    m.body.push("        3'd1: addr_latch[7:4] <= data_in;".into());
    m.body.push("        3'd2: selected <= cm_rom;".into());
    m.body.push("        default: ;".into());
    m.body.push("      endcase".into());
    m.body.push("    end".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// FPGA-safe i4101: 256x4 static RAM. Single clock domain, BSRAM interface.
/// Provides partwise parity with the original MCS-40 chip set.
pub fn chip_i4101_fpga() -> Module {
    let mut m = Module::new("i4101_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("addr").width(8));
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    m.ports.push(Port::input("cs"));
    m.ports.push(Port::input("we"));
    m.ports.push(Port::input("oe"));
    // BSRAM interface
    m.ports.push(Port::output("ram_addr").width(8));
    m.ports.push(Port::input("ram_rdata").width(4));
    m.ports.push(Port::output("ram_wdata").width(4));
    m.ports.push(Port::output("ram_we"));

    m.body.push("assign ram_addr = addr;".into());
    m.body.push("assign ram_wdata = data_in;".into());
    m.body.push("assign ram_we = cs && we;".into());
    m.body
        .push("assign data_out = (cs && oe && !we) ? ram_rdata : 4'd0;".into());

    m
}

/// FPGA-safe i3205: 1-of-8 binary decoder with enable inputs.
/// Pure combinational -- no clock needed, but wrapped in sys_clk domain
/// for consistency. Used for chip-select decoding in multi-chip systems.
pub fn chip_i3205_fpga() -> Module {
    let mut m = Module::new("i3205_fpga");
    m.ports.push(Port::input("addr").width(3));
    m.ports.push(Port::input("e1_n")); // enable 1 (active low)
    m.ports.push(Port::input("e2_n")); // enable 2 (active low)
    m.ports.push(Port::input("e3")); // enable 3 (active high)
    m.ports.push(Port::output("y").width(8)); // active-low decoded outputs

    // Pure combinational: selected output is 0, others are 1
    m.body.push("wire enabled = !e1_n && !e2_n && e3;".into());
    m.body.push("assign y = enabled ? ~(8'd1 << addr) : 8'hFF;".into());

    m
}

/// FPGA-safe i4316: display driver repurposed as virtual 7-segment display.
///
/// Original: drives multiplexed LCD segments with AC backplane.
/// FPGA version: 16-digit segment buffer readable over UART for virtual display.
/// The host terminal renders the digit contents sent as a serial frame.
///
/// Architecture:
///   CPU writes segment data via bus (cs + wr + digit_sel + data_in).
///   A "display changed" flag triggers serialization of the 16-digit buffer
///   into a compact frame sent via an output port (display_data/display_valid).
///   The host-side terminal renders this as 7-segment digits.
pub fn chip_i4316_fpga() -> Module {
    let mut m = Module::new("i4316_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("cs"));
    m.ports.push(Port::input("wr"));
    m.ports.push(Port::input("digit_sel").width(4));
    m.ports.push(Port::input("data_in").width(4));
    // Virtual display output (directly readable by host via UART)
    m.ports.push(Port::output("display_data").width(4));
    m.ports.push(Port::output("display_digit").width(4));
    m.ports.push(Port::output("display_valid"));
    // Original backplane signal (useful for timing/LED blink)
    m.ports.push(Port::output("backplane"));

    m.body
        .push("reg [3:0] segments [0:15]; // 16-digit segment buffer".into());
    m.body.push("reg bp_phase;".into());
    m.body.push("reg [3:0] scan_digit; // multiplex counter".into());
    m.body.push("reg [15:0] refresh_cnt; // refresh timer".into());
    m.body
        .push("reg changed; // set when CPU writes, cleared after scan".into());
    m.body.push("reg scanning; // 1 = outputting digit data".into());
    m.body.push(String::new());

    m.body.push("assign backplane = bp_phase;".into());
    m.body.push("assign display_data = segments[scan_digit];".into());
    m.body.push("assign display_digit = scan_digit;".into());
    m.body.push("assign display_valid = scanning;".into());
    m.body.push(String::new());

    m.body.push("integer di;".into());
    m.body.push("always @(posedge sys_clk) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body.push("    bp_phase <= 1'b0; scan_digit <= 4'd0;".into());
    m.body
        .push("    refresh_cnt <= 16'd0; changed <= 1'b0; scanning <= 1'b0;".into());
    m.body
        .push("    for (di = 0; di < 16; di = di + 1) segments[di] <= 4'd0;".into());
    m.body.push("  end else begin".into());
    // CPU write
    m.body.push("    if (cs && wr) begin".into());
    m.body.push("      segments[digit_sel] <= data_in;".into());
    m.body.push("      changed <= 1'b1;".into());
    m.body.push("    end".into());
    // Refresh scan: when changed, cycle through all 16 digits
    m.body.push("    if (changed && !scanning) begin".into());
    m.body.push("      scanning <= 1'b1; scan_digit <= 4'd0;".into());
    m.body.push("    end else if (scanning) begin".into());
    m.body.push("      if (scan_digit == 4'd15) begin".into());
    m.body.push("        scanning <= 1'b0; changed <= 1'b0;".into());
    m.body.push("        bp_phase <= ~bp_phase;".into());
    m.body.push("      end else begin".into());
    m.body.push("        scan_digit <= scan_digit + 4'd1;".into());
    m.body.push("      end".into());
    m.body.push("    end".into());
    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// FPGA-safe i4269: Programmable Keyboard/Display Interface.
///
/// The crown jewel MCS-40 peripheral. Provides:
///   - Keyboard: 8-character FIFO, interrupt on keypress, debounce
///   - Display: refresh buffer for up to 16 digits (128 segments)
///
/// Simplified for FPGA: directly interfaces with a matrix keyboard
/// (directly on GPIO or via i4003 shift registers) and drives a
/// display buffer accessible via UART.
pub fn chip_i4269_fpga() -> Module {
    let mut m = Module::new("i4269_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("rst"));
    // CPU interface (directly memory-mapped, not bus protocol)
    m.ports.push(Port::input("cs"));
    m.ports.push(Port::input("wr"));
    m.ports.push(Port::input("rd"));
    m.ports.push(Port::input("addr").width(2)); // 4 registers
    m.ports.push(Port::input("data_in").width(4));
    m.ports.push(Port::output("data_out").width(4));
    // Keyboard matrix interface
    m.ports.push(Port::output("key_row").width(4)); // row drive (active low scan)
    m.ports.push(Port::input("key_col").width(4)); // column sense
                                                   // Interrupt output
    m.ports.push(Port::output("int_out"));
    // Display buffer output (virtual display over UART)
    m.ports.push(Port::output("disp_data").width(8));
    m.ports.push(Port::output("disp_addr").width(4));
    m.ports.push(Port::output("disp_valid"));

    // Register map:
    //   addr 0: keyboard data (read: dequeue FIFO, write: control)
    //   addr 1: keyboard status (read: FIFO count[3:0])
    //   addr 2: display data (write: segment data for current digit)
    //   addr 3: display control (write: digit select / mode)

    m.body
        .push("// Keyboard FIFO (8 entries x 8 bits: row[3:0] + col[3:0])".into());
    m.body.push("reg [7:0] key_fifo [0:7];".into());
    m.body.push("reg [2:0] fifo_wr_ptr;".into());
    m.body.push("reg [2:0] fifo_rd_ptr;".into());
    m.body.push("reg [3:0] fifo_count;".into());
    m.body.push(String::new());

    // Keyboard scanner
    m.body.push("reg [3:0] scan_row; // which row is being scanned".into());
    m.body
        .push("reg [15:0] scan_timer; // debounce/scan rate divider".into());
    m.body
        .push("reg [3:0] prev_col; // previous column state for edge detection".into());
    m.body.push(String::new());

    // Display buffer (16 digits x 8 segments)
    m.body.push("reg [7:0] disp_buf [0:15];".into());
    m.body.push("reg [3:0] disp_sel; // current digit for write".into());
    m.body.push("reg disp_changed;".into());
    m.body.push("reg disp_scanning;".into());
    m.body.push("reg [3:0] disp_scan_idx;".into());
    m.body.push(String::new());

    m.body
        .push("assign key_row = ~(4'd1 << scan_row[1:0]); // one-hot active-low".into());
    m.body
        .push("assign int_out = (fifo_count != 4'd0); // interrupt when key available".into());
    m.body.push("assign disp_data = disp_buf[disp_scan_idx];".into());
    m.body.push("assign disp_addr = disp_scan_idx;".into());
    m.body.push("assign disp_valid = disp_scanning;".into());
    m.body.push(String::new());

    // CPU read mux
    m.body
        .push("assign data_out = (addr == 2'd0) ? key_fifo[fifo_rd_ptr][3:0] :".into());
    m.body.push("                  (addr == 2'd1) ? fifo_count :".into());
    m.body
        .push("                  (addr == 2'd2) ? disp_buf[disp_sel][3:0] :".into());
    m.body.push("                  disp_sel;".into());
    m.body.push(String::new());

    m.body.push("integer ki;".into());
    m.body.push("always @(posedge sys_clk) begin".into());
    m.body.push("  if (rst) begin".into());
    m.body
        .push("    fifo_wr_ptr <= 3'd0; fifo_rd_ptr <= 3'd0; fifo_count <= 4'd0;".into());
    m.body
        .push("    scan_row <= 4'd0; scan_timer <= 16'd0; prev_col <= 4'hF;".into());
    m.body.push("    disp_sel <= 4'd0; disp_changed <= 1'b0;".into());
    m.body.push("    disp_scanning <= 1'b0; disp_scan_idx <= 4'd0;".into());
    m.body
        .push("    for (ki = 0; ki < 8; ki = ki + 1) key_fifo[ki] <= 8'd0;".into());
    m.body
        .push("    for (ki = 0; ki < 16; ki = ki + 1) disp_buf[ki] <= 8'd0;".into());
    m.body.push("  end else begin".into());

    // CPU write
    m.body.push("    if (cs && wr) begin".into());
    m.body.push("      case (addr)".into());
    m.body
        .push("        2'd0: ; // control register (future: scan mode, etc.)".into());
    m.body
        .push("        2'd2: begin disp_buf[disp_sel][3:0] <= data_in; disp_changed <= 1'b1; end".into());
    m.body.push("        2'd3: disp_sel <= data_in;".into());
    m.body.push("        default: ;".into());
    m.body.push("      endcase".into());
    m.body.push("    end".into());

    // CPU read: dequeue FIFO on read of addr 0
    m.body
        .push("    if (cs && rd && addr == 2'd0 && fifo_count != 4'd0) begin".into());
    m.body.push("      fifo_rd_ptr <= fifo_rd_ptr + 3'd1;".into());
    m.body.push("      fifo_count <= fifo_count - 4'd1;".into());
    m.body.push("    end".into());

    // Keyboard scanner: advance row, check columns for keypresses
    m.body.push("    scan_timer <= scan_timer + 16'd1;".into());
    m.body
        .push("    if (scan_timer == 16'd0) begin // ~every 65536 sys_clk cycles".into());
    m.body.push("      scan_row <= scan_row + 4'd1;".into());
    m.body.push("      if (scan_row[1:0] == 2'd3) begin".into());
    m.body
        .push("        // Check for new keypresses (falling edge on column)".into());
    m.body
        .push("        if ((prev_col & ~key_col) != 4'd0 && fifo_count < 4'd8) begin".into());
    m.body
        .push("          key_fifo[fifo_wr_ptr] <= {scan_row[1:0], 2'd0, key_col};".into());
    m.body.push("          fifo_wr_ptr <= fifo_wr_ptr + 3'd1;".into());
    m.body.push("          fifo_count <= fifo_count + 4'd1;".into());
    m.body.push("        end".into());
    m.body.push("        prev_col <= key_col;".into());
    m.body.push("      end".into());
    m.body.push("    end".into());

    // Display scan: serialize buffer when changed
    m.body.push("    if (disp_changed && !disp_scanning) begin".into());
    m.body
        .push("      disp_scanning <= 1'b1; disp_scan_idx <= 4'd0;".into());
    m.body.push("    end else if (disp_scanning) begin".into());
    m.body.push("      if (disp_scan_idx == 4'd15) begin".into());
    m.body
        .push("        disp_scanning <= 1'b0; disp_changed <= 1'b0;".into());
    m.body.push("      end else begin".into());
    m.body.push("        disp_scan_idx <= disp_scan_idx + 4'd1;".into());
    m.body.push("      end".into());
    m.body.push("    end".into());

    m.body.push("  end".into());
    m.body.push("end".into());

    m
}

/// FPGA-safe i2102: 1024x1 static RAM. NMOS, TTL-compatible.
/// Used in Intellec-4 program memory modules. BSRAM-backed.
pub fn chip_i2102_fpga() -> Module {
    let mut m = Module::new("i2102_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("rst"));
    m.ports.push(Port::input("addr").width(10));
    m.ports.push(Port::input("data_in"));
    m.ports.push(Port::output("data_out"));
    m.ports.push(Port::input("ce_n"));
    m.ports.push(Port::input("rw")); // 1=read, 0=write
                                     // BSRAM interface (1-bit wide, 1K deep)
    m.ports.push(Port::output("ram_addr").width(10));
    m.ports.push(Port::input("ram_rdata"));
    m.ports.push(Port::output("ram_wdata"));
    m.ports.push(Port::output("ram_we"));

    m.body.push("assign ram_addr = addr;".into());
    m.body.push("assign ram_wdata = data_in;".into());
    m.body.push("assign ram_we = !ce_n && !rw;".into());
    m.body
        .push("assign data_out = (!ce_n && rw) ? ram_rdata : 1'b0;".into());

    m
}

/// FPGA-safe i1302: 256x8 mask-programmed ROM. PMOS.
/// Production ROM variant of the i1702 EPROM family. BSRAM-backed.
pub fn chip_i1302_fpga() -> Module {
    let mut m = Module::new("i1302_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("addr").width(8));
    m.ports.push(Port::output("data_out").width(8));
    m.ports.push(Port::input("cs_n")); // chip select (active low)
                                       // BSRAM interface
    m.ports.push(Port::output("rom_addr").width(8));
    m.ports.push(Port::input("rom_data").width(8));

    m.body.push("assign rom_addr = addr;".into());
    m.body.push("assign data_out = !cs_n ? rom_data : 8'hFF;".into());

    m
}

/// FPGA-safe i2316: 2048x8 mask ROM. NMOS, TTL-compatible.
/// Large ROM used in Intellec systems and many microcomputers (Apple, Commodore).
/// 3 programmable chip selects allow 8 chips OR-tied without external decoding.
pub fn chip_i2316_fpga() -> Module {
    let mut m = Module::new("i2316_fpga");
    m.ports.push(Port::input("sys_clk"));
    m.ports.push(Port::input("addr").width(11));
    m.ports.push(Port::output("data_out").width(8));
    m.ports.push(Port::input("cs1_n")); // chip select 1 (active low)
    m.ports.push(Port::input("cs2_n")); // chip select 2 (active low)
    m.ports.push(Port::input("cs3")); // chip select 3 (active high)
                                      // BSRAM interface
    m.ports.push(Port::output("rom_addr").width(11));
    m.ports.push(Port::input("rom_data").width(8));

    m.body.push("wire selected = !cs1_n && !cs2_n && cs3;".into());
    m.body.push("assign rom_addr = addr;".into());
    m.body.push("assign data_out = selected ? rom_data : 8'hFF;".into());

    m
}

/// Generate all MCS-4/MCS-40 chip modules (original tristate bus versions).
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

/// Generate FPGA-safe chip modules (split bus, no internal tristates).
///
/// These variants are designed for FPGA synthesis where internal tristate
/// buses are not supported. Each chip uses data_in/data_out/data_oe ports
/// instead of inout data.
pub fn fpga_chip_modules() -> Vec<Module> {
    vec![
        // MCS-4 core
        chip_i4004_fpga(), // 4-bit CPU (46 instructions)
        chip_i4001_fpga(), // 256x8 ROM + I/O port
        chip_i4002_fpga(), // 320-bit RAM + output port
        chip_i4003_fpga(), // 10-bit shift register
        // MCS-4 support
        chip_i3205_fpga(), // 1-of-8 decoder (chip select)
        chip_i4101_fpga(), // 256x4 SRAM (BSRAM-backed)
        // MCS-40 enhanced
        chip_i4040_fpga(), // Enhanced CPU (60 instructions, interrupts)
        chip_i4308_fpga(), // 1Kx8 ROM + I/O port
        chip_i4265_fpga(), // Programmable I/O (4x4 bits)
        chip_i4702_fpga(), // 256x8 EPROM
        // MCS-40 peripherals
        chip_i4316_fpga(), // Display driver (virtual 7-seg over UART)
        chip_i4269_fpga(), // Keyboard/Display interface (FIFO + scanner)
        // Intellec/memory chips
        chip_i2102_fpga(), // 1Kx1 SRAM (Intellec program memory)
        chip_i1302_fpga(), // 256x8 mask ROM
        chip_i2316_fpga(), // 2Kx8 mask ROM (large ROM)
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

    // --- Verilog file generation (run with --ignored) ---

    #[test]
    #[ignore = "codegen helper writing build/*.v; run explicitly with --ignored"]
    fn generate_fpga_verilog() {
        use std::{fs, path::Path};

        let build_dir = Path::new("build");
        fs::create_dir_all(build_dir).unwrap();

        let exporter = VerilogExporter::new("unused");
        for module in fpga_chip_modules() {
            let path = build_dir.join(format!("{}.v", module.name));
            let mut f = fs::File::create(&path).unwrap();
            exporter.export_module(&module, &mut f).unwrap();
            eprintln!("Generated: {}", path.display());
        }
    }

    // --- FPGA-safe split-bus variant tests ---

    #[test]
    fn i4004_fpga_has_split_bus() {
        let m = chip_i4004_fpga();
        assert_eq!(m.name, "i4004_fpga");
        let port_names: Vec<&str> = m.ports.iter().map(|p| p.name.as_str()).collect();

        // Must have split bus ports
        assert!(port_names.contains(&"data_in"), "missing data_in");
        assert!(port_names.contains(&"data_out"), "missing data_out");
        assert!(port_names.contains(&"data_oe"), "missing data_oe");

        // Must NOT have inout data
        assert!(!port_names.contains(&"data"), "FPGA variant must not have inout data");

        // Verify directions
        let di = m.ports.iter().find(|p| p.name == "data_in").unwrap();
        assert_eq!(di.dir, PortDir::Input);
        assert_eq!(di.width, 4);
        let dout = m.ports.iter().find(|p| p.name == "data_out").unwrap();
        assert_eq!(dout.dir, PortDir::Output);
        assert_eq!(dout.width, 4);
        let doe = m.ports.iter().find(|p| p.name == "data_oe").unwrap();
        assert_eq!(doe.dir, PortDir::Output);
        assert_eq!(doe.width, 1);
    }

    #[test]
    fn i4004_fpga_renders_no_tristate() {
        let m = chip_i4004_fpga();
        let v = render_module(&m);
        assert!(v.contains("module i4004_fpga"));
        assert!(!v.contains("4'bz"), "FPGA variant must not use tristate");
        assert!(!v.contains("inout"), "FPGA variant must not have inout");
        assert!(v.contains("assign data_oe"));
        assert!(v.contains("assign data_out"));
        assert!(v.contains("endmodule"));
    }

    #[test]
    fn i4004_fpga_has_instruction_decode() {
        let m = chip_i4004_fpga();
        let v = render_module(&m);
        // Verify key 4004 instructions are implemented
        assert!(v.contains("NOP"), "missing NOP");
        assert!(v.contains("LDM"), "missing LDM");
        assert!(v.contains("IAC"), "missing IAC");
        assert!(v.contains("RAL"), "missing RAL");
        assert!(v.contains("RAR"), "missing RAR");
        assert!(v.contains("CMA"), "missing CMA");
        assert!(v.contains("KBP"), "missing KBP");
        assert!(v.contains("DAA"), "missing DAA");
    }

    #[test]
    fn i4001_fpga_has_split_bus_and_bsram_port() {
        let m = chip_i4001_fpga();
        assert_eq!(m.name, "i4001_fpga");
        let port_names: Vec<&str> = m.ports.iter().map(|p| p.name.as_str()).collect();

        // Split bus
        assert!(port_names.contains(&"data_in"));
        assert!(port_names.contains(&"data_out"));
        assert!(port_names.contains(&"data_oe"));
        assert!(!port_names.contains(&"data"));

        // BSRAM interface
        assert!(port_names.contains(&"rom_addr"), "missing rom_addr for BSRAM");
        assert!(port_names.contains(&"rom_data"), "missing rom_data for BSRAM");

        // I/O ports
        assert!(port_names.contains(&"io_out"));
        assert!(port_names.contains(&"io_in"));
        assert!(port_names.contains(&"io_wr"));

        let v = render_module(&m);
        assert!(
            !v.contains("reg [7:0] rom"),
            "FPGA variant should not have internal ROM array"
        );
    }

    #[test]
    fn i4002_fpga_has_split_bus_and_ram_port() {
        let m = chip_i4002_fpga();
        assert_eq!(m.name, "i4002_fpga");
        let port_names: Vec<&str> = m.ports.iter().map(|p| p.name.as_str()).collect();

        // Split bus
        assert!(port_names.contains(&"data_in"));
        assert!(port_names.contains(&"data_out"));
        assert!(port_names.contains(&"data_oe"));
        assert!(!port_names.contains(&"data"));

        // External RAM port
        assert!(port_names.contains(&"ram_addr"), "missing ram_addr");
        assert!(port_names.contains(&"ram_rdata"), "missing ram_rdata");
        assert!(port_names.contains(&"ram_wdata"), "missing ram_wdata");
        assert!(port_names.contains(&"ram_we"), "missing ram_we");
    }

    #[test]
    fn fpga_modules_all_render() {
        let modules = fpga_chip_modules();
        assert_eq!(modules.len(), 15);
        for module in &modules {
            let v = render_module(module);
            assert!(
                v.contains(&format!("module {}", module.name)),
                "Module {} missing header",
                module.name
            );
            assert!(v.contains("endmodule"));
            assert!(!v.contains("inout"), "FPGA module {} has inout (tristate)", module.name);
        }
    }

    #[test]
    fn i4003_fpga_shift_register() {
        let m = chip_i4003_fpga();
        assert_eq!(m.name, "i4003_fpga");
        let v = render_module(&m);
        assert!(v.contains("shift_reg"));
        assert!(v.contains("serial_out"));
        assert!(v.contains("parallel_out"));
        assert!(v.contains("sys_clk"));
    }

    #[test]
    fn i4040_fpga_has_7_level_stack() {
        let m = chip_i4040_fpga();
        assert_eq!(m.name, "i4040_fpga");
        let v = render_module(&m);
        assert!(v.contains("stack [0:6]"), "4040 needs 7-level stack");
        assert!(v.contains("int_enabled"), "4040 needs interrupt support");
        assert!(v.contains("reg_bank"), "4040 needs register bank select");
        assert!(v.contains("halted"), "4040 needs HLT support");
        assert!(v.contains("regs [0:23]"), "4040 needs 24 registers");
        assert!(v.contains("HLT"), "4040 needs HLT instruction");
        assert!(v.contains("EIN"), "4040 needs EIN instruction");
    }

    #[test]
    fn i4308_fpga_has_10bit_address() {
        let m = chip_i4308_fpga();
        assert_eq!(m.name, "i4308_fpga");
        let v = render_module(&m);
        assert!(v.contains("[9:0] addr_latch"), "4308 needs 10-bit address");
        let addr_port = m.ports.iter().find(|p| p.name == "rom_addr").unwrap();
        assert_eq!(addr_port.width, 10, "rom_addr must be 10 bits wide");
    }

    #[test]
    fn i4265_fpga_has_4_io_ports() {
        let m = chip_i4265_fpga();
        assert_eq!(m.name, "i4265_fpga");
        let port_names: Vec<&str> = m.ports.iter().map(|p| p.name.as_str()).collect();
        assert!(port_names.contains(&"io_0_out"));
        assert!(port_names.contains(&"io_3_in"));
        assert!(port_names.contains(&"io_2_oe"));
        let v = render_module(&m);
        assert!(v.contains("port_dir"), "4265 needs direction registers");
    }

    #[test]
    fn i4702_fpga_is_eprom() {
        let m = chip_i4702_fpga();
        assert_eq!(m.name, "i4702_fpga");
        let v = render_module(&m);
        assert!(v.contains("rom_addr"));
        assert!(v.contains("rom_data"));
        assert!(v.contains("cm_rom"));
        assert!(!v.contains("inout"));
    }

    #[test]
    fn i4101_fpga_sram_with_bsram() {
        let m = chip_i4101_fpga();
        assert_eq!(m.name, "i4101_fpga");
        let port_names: Vec<&str> = m.ports.iter().map(|p| p.name.as_str()).collect();
        assert!(port_names.contains(&"ram_addr"), "needs BSRAM address port");
        assert!(port_names.contains(&"ram_rdata"), "needs BSRAM read data");
        assert!(port_names.contains(&"ram_wdata"), "needs BSRAM write data");
        assert!(port_names.contains(&"ram_we"), "needs BSRAM write enable");
        assert!(port_names.contains(&"cs"), "needs chip select");
        assert!(port_names.contains(&"we"), "needs write enable");
        assert!(port_names.contains(&"oe"), "needs output enable");
    }

    #[test]
    fn i3205_fpga_decoder() {
        let m = chip_i3205_fpga();
        assert_eq!(m.name, "i3205_fpga");
        let v = render_module(&m);
        assert!(v.contains("8'd1 << addr"), "should decode to one-hot");
        assert!(v.contains("e1_n"), "needs enable inputs");
        assert!(!v.contains("sys_clk"), "decoder is pure combinational");
    }

    #[test]
    fn i4316_fpga_virtual_display() {
        let m = chip_i4316_fpga();
        assert_eq!(m.name, "i4316_fpga");
        let v = render_module(&m);
        let port_names: Vec<&str> = m.ports.iter().map(|p| p.name.as_str()).collect();
        assert!(port_names.contains(&"display_data"), "needs display data output");
        assert!(port_names.contains(&"display_digit"), "needs digit address output");
        assert!(port_names.contains(&"display_valid"), "needs valid strobe");
        assert!(port_names.contains(&"backplane"), "needs backplane signal");
        assert!(v.contains("segments"), "needs segment buffer");
    }

    #[test]
    fn i4269_fpga_keyboard_display() {
        let m = chip_i4269_fpga();
        assert_eq!(m.name, "i4269_fpga");
        let v = render_module(&m);
        let port_names: Vec<&str> = m.ports.iter().map(|p| p.name.as_str()).collect();
        // Keyboard
        assert!(port_names.contains(&"key_row"), "needs keyboard row drive");
        assert!(port_names.contains(&"key_col"), "needs keyboard column sense");
        assert!(port_names.contains(&"int_out"), "needs interrupt output");
        assert!(v.contains("key_fifo"), "needs keyboard FIFO");
        assert!(v.contains("fifo_count"), "needs FIFO count");
        // Display
        assert!(port_names.contains(&"disp_data"), "needs display data output");
        assert!(port_names.contains(&"disp_addr"), "needs display address");
        assert!(port_names.contains(&"disp_valid"), "needs display valid strobe");
        assert!(v.contains("disp_buf"), "needs display buffer");
    }

    #[test]
    fn i2102_fpga_1kx1_sram() {
        let m = chip_i2102_fpga();
        assert_eq!(m.name, "i2102_fpga");
        let port_names: Vec<&str> = m.ports.iter().map(|p| p.name.as_str()).collect();
        assert!(port_names.contains(&"ram_addr"));
        let addr = m.ports.iter().find(|p| p.name == "addr").unwrap();
        assert_eq!(addr.width, 10, "2102 needs 10-bit address for 1K");
        let din = m.ports.iter().find(|p| p.name == "data_in").unwrap();
        assert_eq!(din.width, 1, "2102 is 1-bit wide");
    }

    #[test]
    fn i1302_fpga_mask_rom() {
        let m = chip_i1302_fpga();
        assert_eq!(m.name, "i1302_fpga");
        let port_names: Vec<&str> = m.ports.iter().map(|p| p.name.as_str()).collect();
        assert!(port_names.contains(&"rom_addr"));
        assert!(port_names.contains(&"rom_data"));
        assert!(port_names.contains(&"cs_n"));
        let dout = m.ports.iter().find(|p| p.name == "data_out").unwrap();
        assert_eq!(dout.width, 8, "1302 is 8-bit wide");
    }

    #[test]
    fn i2316_fpga_large_rom() {
        let m = chip_i2316_fpga();
        assert_eq!(m.name, "i2316_fpga");
        let addr = m.ports.iter().find(|p| p.name == "addr").unwrap();
        assert_eq!(addr.width, 11, "2316 needs 11-bit address for 2K");
        let v = render_module(&m);
        assert!(v.contains("cs1_n"), "needs 3 chip selects");
        assert!(v.contains("cs2_n"));
        assert!(v.contains("cs3"));
    }
}
