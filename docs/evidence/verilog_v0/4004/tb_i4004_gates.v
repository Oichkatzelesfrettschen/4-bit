// Testbench unavailable for an incomplete gate-HDL export.
// Module: i4004_gates
// Reason: output CLK1 reaches n415 with 128 drivers

`timescale 1ns/1ps

module tb_i4004_gates;
    initial begin
        $fatal(1, "i4004_gates is not delivery-ready: output CLK1 reaches n415 with 128 drivers");
    end
endmodule
