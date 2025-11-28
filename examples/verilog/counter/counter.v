// Simple 8-bit Counter for NovyWave Demonstration
// This example generates a VCD waveform file using Icarus Verilog

module counter (
    input  wire       clk,
    input  wire       reset,
    input  wire       enable,
    output reg  [7:0] count,
    output reg        overflow
);

    always @(posedge clk or posedge reset) begin
        if (reset) begin
            count <= 8'b0;
            overflow <= 1'b0;
        end else if (enable) begin
            if (count == 8'hFF) begin
                count <= 8'b0;
                overflow <= 1'b1;
            end else begin
                count <= count + 1'b1;
                overflow <= 1'b0;
            end
        end
    end

endmodule
