// Testbench unavailable for an incomplete gate-HDL export.
// Module: i4001_gates
// Reason: output CL reaches undriven nodes n1236

`timescale 1ns/1ps

module tb_i4001_gates;
    initial begin
        $fatal(1, "i4001_gates is not delivery-ready: output CL reaches undriven nodes n1236");
    end
endmodule
