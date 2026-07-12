// Testbench unavailable for an incomplete gate-HDL export.
// Module: i4002_gates
// Reason: no exported output port derives from signal anchors

`timescale 1ns/1ps

module tb_i4002_gates;
    initial begin
        $fatal(1, "i4002_gates is not delivery-ready: no exported output port derives from signal anchors");
    end
endmodule
