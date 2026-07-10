// Testbench (auto-generated)
// Module: i4002_gates
// Two-phase non-overlapping clock, reset pulse, walking data pattern

`timescale 1ns/1ps

module tb_i4002_gates;
    reg VDD, VSS;
    reg CM;

    i4002_gates dut (
        .VDD(VDD),
        .VSS(VSS),
        .CM(CM)
    );

    // Two-phase non-overlapping clock, 1350 ns period:
    // phi1 high 0-540, dead 540-675, phi2 high 675-1215, dead 1215-1350.
    integer cycle;
    initial begin
        VDD = 1;
        VSS = 0;
        CM = 0;

        for (cycle = 0; cycle < 32; cycle = cycle + 1) begin
            #540;
            #135;
            #540;
            #135;
            // Walk the data inputs so every one toggles
            CM = (cycle >> 0) & 1;
        end

        $finish;
    end

    // Waveform dump
    initial begin
        $dumpfile("tb_i4002_gates.vcd");
        $dumpvars(0, tb_i4002_gates);
    end
endmodule
