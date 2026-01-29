// Testbench template (auto-generated)
// Module: i4004_gates

`timescale 1ns/1ps

module tb_i4004_gates;
    // DUT signals
    reg VDD, VSS;

    // DUT instantiation
    i4004_gates dut (
        .VDD(VDD),
        .VSS(VSS)
    );

    // Initialize
    initial begin
        VDD = 1;
        VSS = 0;

        // TODO: Add test stimulus

        #10000 $finish;
    end

    // Waveform dump
    initial begin
        $dumpfile("tb_i4004_gates.vcd");
        $dumpvars(0, tb_i4004_gates);
    end
endmodule
