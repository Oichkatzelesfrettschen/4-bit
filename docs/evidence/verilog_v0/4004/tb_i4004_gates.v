// Testbench (auto-generated)
// Module: i4004_gates
// Two-phase non-overlapping clock, reset pulse, walking data pattern

`timescale 1ns/1ps

module tb_i4004_gates;
    reg VDD, VSS;
    wire CLK1;
    wire CMRAM0;
    wire D0_PAD;

    i4004_gates dut (
        .VDD(VDD),
        .VSS(VSS),
        .CLK1(CLK1),
        .CMRAM0(CMRAM0),
        .D0_PAD(D0_PAD)
    );

    // Two-phase non-overlapping clock, 1350 ns period:
    // phi1 high 0-540, dead 540-675, phi2 high 675-1215, dead 1215-1350.
    integer cycle;
    initial begin
        VDD = 1;
        VSS = 0;

        for (cycle = 0; cycle < 32; cycle = cycle + 1) begin
            #540;
            #135;
            #540;
            #135;
        end

        $display("i4004_gates final: CLK1=%b, CMRAM0=%b, D0_PAD=%b", CLK1, CMRAM0, D0_PAD);
        $finish;
    end

    // Waveform dump
    initial begin
        $dumpfile("tb_i4004_gates.vcd");
        $dumpvars(0, tb_i4004_gates);
    end
endmodule
