// Testbench (auto-generated)
// Module: i4003_gates
// Exhaustive binary input vectors with an X/Z resolution oracle.
// This testbench does not establish chip-level functional equivalence.

`timescale 1ns/1ps

module tb_i4003_gates;
    reg VDD, VSS;
    reg Q2;
    reg Q5;
    reg Q6;
    wire Q4;

    task require_known;
        input value;
        input [8*64-1:0] signal_name;
        begin
            if ((value !== 1'b0) && (value !== 1'b1)) begin
                $display("FAIL: unresolved output %0s=%b", signal_name, value);
                $fatal(1);
            end
        end
    endtask

    i4003_gates dut (
        .VDD(VDD),
        .VSS(VSS),
        .Q2(Q2),
        .Q5(Q5),
        .Q6(Q6),
        .Q4(Q4)
    );

    integer vector;
    initial begin
        VDD = 1'b1;
        VSS = 1'b0;
        for (vector = 0; vector < 8; vector = vector + 1) begin
            Q2 = (vector >> 0) & 1'b1;
            Q5 = (vector >> 1) & 1'b1;
            Q6 = (vector >> 2) & 1'b1;
            #1;
            require_known(Q4, "Q4");
        end
        $display("PASS: i4003_gates resolves all 8 input vectors");
        $finish;
    end

    initial begin
        $dumpfile("tb_i4003_gates.vcd");
        $dumpvars(0, tb_i4003_gates);
    end
endmodule
