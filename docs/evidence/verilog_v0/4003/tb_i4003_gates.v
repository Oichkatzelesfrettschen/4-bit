// Testbench (auto-generated)
// Module: i4003_gates
// Two-phase non-overlapping clock, reset pulse, walking data pattern

`timescale 1ns/1ps

module tb_i4003_gates;
    reg VDD, VSS;
    reg Q2;
    reg Q5;
    reg Q6;
    wire Q4;

    i4003_gates dut (
        .VDD(VDD),
        .VSS(VSS),
        .Q2(Q2),
        .Q5(Q5),
        .Q6(Q6),
        .Q4(Q4)
    );

    // Two-phase non-overlapping clock, 1350 ns period:
    // phi1 high 0-540, dead 540-675, phi2 high 675-1215, dead 1215-1350.
    integer cycle;
    initial begin
        VDD = 1;
        VSS = 0;
        Q2 = 0;
        Q5 = 1;
        Q6 = 0;

        for (cycle = 0; cycle < 32; cycle = cycle + 1) begin
            #540;
            #135;
            #540;
            #135;
            // Walk the data inputs so every one toggles
            Q2 = (cycle >> 0) & 1;
            Q5 = (cycle >> 1) & 1;
            Q6 = (cycle >> 2) & 1;
        end

        $display("i4003_gates final: Q4=%b", Q4);
        $finish;
    end

    // Waveform dump
    initial begin
        $dumpfile("tb_i4003_gates.vcd");
        $dumpvars(0, tb_i4003_gates);
    end
endmodule
