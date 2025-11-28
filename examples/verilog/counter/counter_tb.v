// Testbench for 8-bit Counter
// Generates a VCD waveform file for viewing in NovyWave

`timescale 1ns/1ps

module counter_tb;
    // Clock period: 10ns (100 MHz)
    parameter CLK_PERIOD = 10;

    reg        clk;
    reg        reset;
    reg        enable;
    wire [7:0] count;
    wire       overflow;

    // Instantiate the counter
    counter uut (
        .clk(clk),
        .reset(reset),
        .enable(enable),
        .count(count),
        .overflow(overflow)
    );

    // Clock generation
    initial begin
        clk = 0;
        forever #(CLK_PERIOD/2) clk = ~clk;
    end

    // VCD dump for waveform viewing
    initial begin
        $dumpfile("counter.vcd");
        $dumpvars(0, counter_tb);
    end

    // Stimulus
    initial begin
        // Initial reset
        reset = 1;
        enable = 0;
        #50;

        // Release reset, enable counting
        reset = 0;
        enable = 1;
        #300;

        // Disable counting briefly
        enable = 0;
        #50;

        // Resume counting
        enable = 1;
        #200;

        // Apply reset while counting
        reset = 1;
        #30;
        reset = 0;
        #200;

        // Continue until overflow
        #3000;

        // End simulation
        $display("Simulation complete!");
        $finish;
    end

endmodule
