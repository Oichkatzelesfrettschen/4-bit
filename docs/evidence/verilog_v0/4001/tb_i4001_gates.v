// Testbench (auto-generated)
// Module: i4001_gates
// Two-phase non-overlapping clock, reset pulse, walking data pattern

`timescale 1ns/1ps

module tb_i4001_gates;
    reg VDD, VSS;
    reg D0;
    reg IO0;
    wire CL;
    wire CM;
    wire D2;
    wire D3;

    i4001_gates dut (
        .VDD(VDD),
        .VSS(VSS),
        .D0(D0),
        .IO0(IO0),
        .CL(CL),
        .CM(CM),
        .D2(D2),
        .D3(D3)
    );

    // Two-phase non-overlapping clock, 1350 ns period:
    // phi1 high 0-540, dead 540-675, phi2 high 675-1215, dead 1215-1350.
    integer cycle;
    initial begin
        VDD = 1;
        VSS = 0;
        D0 = 0;
        IO0 = 1;

        for (cycle = 0; cycle < 32; cycle = cycle + 1) begin
            #540;
            #135;
            #540;
            #135;
            // Walk the data inputs so every one toggles
            D0 = (cycle >> 0) & 1;
            IO0 = (cycle >> 1) & 1;
        end

        $display("i4001_gates final: CL=%b, CM=%b, D2=%b, D3=%b", CL, CM, D2, D3);
        $finish;
    end

    // Waveform dump
    initial begin
        $dumpfile("tb_i4001_gates.vcd");
        $dumpvars(0, tb_i4001_gates);
    end
endmodule
