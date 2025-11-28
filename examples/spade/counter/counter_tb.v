// Testbench for Spade-generated counter
// Uses Icarus Verilog for simulation and VCD generation

`timescale 1ns/1ps

module counter_tb;
    // Signals
    reg clk;
    reg rst;
    reg enable;
    wire [8:0] output_packed;

    // Extract count and overflow from packed output
    // Spade packs (uint<8>, bool) as {overflow[8], count[7:0]}
    wire [7:0] count;
    wire overflow;
    assign count = output_packed[7:0];
    assign overflow = output_packed[8];

    // Instantiate Spade-generated counter
    top dut (
        .clk(clk),
        .rst(rst),
        .enable(enable),
        .output__(output_packed)
    );

    // Clock generation: 100 MHz (10ns period)
    initial begin
        clk = 0;
        forever #5 clk = ~clk;
    end

    // VCD dump for NovyWave
    initial begin
        $dumpfile("counter.vcd");
        $dumpvars(0, counter_tb);
    end

    // Test sequence
    initial begin
        // Initialize
        rst = 1;
        enable = 0;
        #100;  // Wait 100ns

        // Release reset
        rst = 0;
        #50;

        // Test 1: Enable counting
        $display("Test 1: Enable counting");
        enable = 1;
        #200;

        // Test 2: Disable counting
        $display("Test 2: Disable counting");
        enable = 0;
        #50;

        // Test 3: Resume counting
        $display("Test 3: Resume counting");
        enable = 1;
        #100;

        // Test 4: Count to overflow
        $display("Test 4: Counting to overflow");
        // Continue until overflow
        wait(overflow);
        $display("  Overflow detected!");
        #100;

        // Test 5: Reset during operation
        $display("Test 5: Reset during operation");
        rst = 1;
        #30;
        rst = 0;
        #100;

        $display("Simulation complete!");
        $finish;
    end

    // Timeout watchdog
    initial begin
        #50000;
        $display("Timeout reached");
        $finish;
    end
endmodule
