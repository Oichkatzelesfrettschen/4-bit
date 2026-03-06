// Auto-generated from gate-level netlist
// Chip: 4003
// Tool: gate_to_verilog_v0.py

module i4003_gates (
    input wire VDD,
    input wire VSS
);

    // Internal wires
    wire n7;
    wire n42;
    wire n58;
    wire n65;
    wire n68;
    wire n74;
    wire n78;
    wire n86;
    wire n94;
    wire n110;
    wire n290;
    wire n297;
    wire n303;
    wire n377;
    wire n389;
    wire n415;

    // Gate instances
    nand2 g0 (.A(n65), .B(n42), .Y(n290));
    nand2 g1 (.A(n58), .B(n74), .Y(n297));
    nand2 g2 (.A(n42), .B(n68), .Y(n303));
    nand2 g3 (.A(n78), .B(n7), .Y(n389));
    nand2 g4 (.A(n94), .B(n86), .Y(n377));
    nand2 g5 (.A(n7), .B(n110), .Y(n415));

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
