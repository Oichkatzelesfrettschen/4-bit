// Testbench (auto-generated)
// Module: i4001_gates
// Two-phase non-overlapping clock, reset pulse, walking data pattern

`timescale 1ns/1ps

module tb_i4001_gates;
    reg VDD, VSS;
    reg D0_PAD;
    reg IO0;
    wire CL;
    wire CM;
    wire D2_PAD;
    wire D3_PAD;

    i4001_gates dut (
        .VDD(VDD),
        .VSS(VSS),
        .D0_PAD(D0_PAD),
        .IO0(IO0),
        .CL(CL),
        .CM(CM),
        .D2_PAD(D2_PAD),
        .D3_PAD(D3_PAD)
    );

    // Two-phase non-overlapping clock, 1350 ns period:
    // phi1 high 0-540, dead 540-675, phi2 high 675-1215, dead 1215-1350.
    integer cycle;
    initial begin
        VDD = 1;
        VSS = 0;
        D0_PAD = 0;
        IO0 = 1;

        for (cycle = 0; cycle < 32; cycle = cycle + 1) begin
            #540;
            #135;
            #540;
            #135;
            // Walk the data inputs so every one toggles
            D0_PAD = (cycle >> 0) & 1;
            IO0 = (cycle >> 1) & 1;
        end

        $display("i4001_gates final: CL=%b, CM=%b, D2_PAD=%b, D3_PAD=%b", CL, CM, D2_PAD, D3_PAD);
        $finish;
    end

    // Waveform dump
    initial begin
        $dumpfile("tb_i4001_gates.vcd");
        $dumpvars(0, tb_i4001_gates);
    end
endmodule
