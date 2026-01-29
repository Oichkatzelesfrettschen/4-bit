// Testbench template (auto-generated)
// Module: i4001_gates

`timescale 1ns/1ps

module tb_i4001_gates;
    // DUT signals
    reg VDD, VSS;

    // DUT instantiation
    i4001_gates dut (
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
        $dumpfile("tb_i4001_gates.vcd");
        $dumpvars(0, tb_i4001_gates);
    end
endmodule
