// Auto-generated from gate-level netlist
// Chip: 4001
// Tool: gate_to_verilog_v0.py

module i4001_gates (
    input wire VDD,
    input wire VSS
);

    // Internal wires

    // Gate instances

endmodule

// ========================================
// Primitive Gate Library
// ========================================

// Inverter
module inv (
    input wire A,
    output wire Y
);
    assign Y = ~A;
endmodule

// 2-input NAND
module nand2 (
    input wire A,
    input wire B,
    output wire Y
);
    assign Y = ~(A & B);
endmodule

// 3-input NAND
module nand3 (
    input wire A,
    input wire B,
    input wire C,
    output wire Y
);
    assign Y = ~(A & B & C);
endmodule

// 2-input NOR
module nor2 (
    input wire A,
    input wire B,
    output wire Y
);
    assign Y = ~(A | B);
endmodule

// 3-input NOR
module nor3 (
    input wire A,
    input wire B,
    input wire C,
    output wire Y
);
    assign Y = ~(A | B | C);
endmodule

// Transmission gate (bidirectional)
module tgate (
    input wire EN,
    input wire ENB,
    inout wire A,
    inout wire B
);
    assign A = EN ? B : 1'bz;
    assign B = EN ? A : 1'bz;
endmodule
