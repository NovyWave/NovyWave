# Verilog with Icarus

This tutorial shows how to generate waveforms from Verilog using Icarus Verilog and view them in NovyWave.

## Prerequisites

- [Icarus Verilog](http://iverilog.icarus.com/) installed
- NovyWave installed

### Installing Icarus Verilog

**Ubuntu/Debian:**
```bash
sudo apt-get install iverilog
```

**macOS:**
```bash
brew install icarus-verilog
```

**Windows:**
Download from [Icarus Verilog website](http://iverilog.icarus.com/)

## Step 1: Create a Simple Design

Create `counter.v`:

```verilog
module counter (
    input  wire       clk,
    input  wire       reset,
    output reg [7:0]  count
);

    always @(posedge clk or posedge reset) begin
        if (reset)
            count <= 8'b0;
        else
            count <= count + 1;
    end

endmodule
```

## Step 2: Create a Testbench

Create `counter_tb.v`:

```verilog
`timescale 1ns/1ps

module counter_tb;

    reg        clk;
    reg        reset;
    wire [7:0] count;

    // Instantiate the design
    counter dut (
        .clk(clk),
        .reset(reset),
        .count(count)
    );

    // Clock generation
    initial begin
        clk = 0;
        forever #5 clk = ~clk;  // 100MHz clock (10ns period)
    end

    // Waveform dump
    initial begin
        $dumpfile("counter.vcd");
        $dumpvars(0, counter_tb);
    end

    // Stimulus
    initial begin
        reset = 1;
        #25;
        reset = 0;
        #3000;
        $finish;
    end

endmodule
```

## Step 3: Compile the Design

```bash
iverilog -o counter_sim counter.v counter_tb.v
```

## Step 4: Run Simulation

```bash
vvp counter_sim
```

This creates `counter.vcd` with all signal transitions.

## Step 5: View in NovyWave

1. Open NovyWave
2. Click **Load Files**
3. Select `counter.vcd`
4. Click **Load**

The file appears in Files & Scopes:

```
📄 counter.vcd (0-3025ns)
  └── 📁 counter_tb
      └── 📁 dut
```

## Step 6: Select Signals

1. Click checkbox next to `counter_tb`
2. In Variables panel, click:
   - `clk`
   - `reset`
   - `count`

## Step 7: Explore the Waveform

- Press `R` for full view
- Zoom in with `W` to see the reset release at 25ns
- Use `Shift+E` to jump between counter transitions
- Change `count` format to UInt to see decimal values

## Verilog Waveform Commands

### Basic Dump

```verilog
initial begin
    $dumpfile("output.vcd");
    $dumpvars(0, testbench_name);  // Dump all signals
end
```

### Selective Dump

```verilog
initial begin
    $dumpfile("output.vcd");
    $dumpvars(1, testbench_name);        // Only top level
    $dumpvars(0, testbench_name.dut);    // All signals in dut
end
```

### Dump Control

```verilog
initial begin
    $dumpfile("output.vcd");
    $dumpvars(0, testbench_name);

    #1000;
    $dumpoff;   // Stop dumping
    #500;
    $dumpon;    // Resume dumping
    #1000;
    $dumpflush; // Flush to file
end
```

## Using FST Format (Verilator)

For better performance with large designs, use Verilator with FST output:

```cpp
// In C++ testbench
#include "verilated_fst_c.h"

int main() {
    Verilated::traceEverOn(true);

    VerilatedFstC* tfp = new VerilatedFstC;
    top->trace(tfp, 99);
    tfp->open("output.fst");

    // ... simulation loop ...

    tfp->close();
}
```

FST files are 10-100x smaller and faster to load in NovyWave.

## Common Patterns

### Clock and Reset

```verilog
// Standard testbench pattern
reg clk = 0;
reg reset = 1;

always #5 clk = ~clk;

initial begin
    #20 reset = 0;  // Release reset after 20ns
end
```

### Parameterized Timing

```verilog
parameter CLK_PERIOD = 10;

always #(CLK_PERIOD/2) clk = ~clk;
```

### Automatic Test Termination

```verilog
initial begin
    // ... stimulus ...

    repeat(100) @(posedge clk);  // Wait 100 cycles
    $finish;
end
```

## Next Steps

- Add assertions to your testbench
- Create parameterized tests
- Compare multiple test runs with [multi-file tutorial](./multi-file.md)
- Try SystemVerilog features (if using supported simulator)

## Troubleshooting

### No VCD file created
- Ensure `$dumpfile` and `$dumpvars` are called before signals change
- Check simulation runs to completion (look for `$finish`)

### Missing signals
- Verify module hierarchy in `$dumpvars` call
- Use depth 0 to dump all signals: `$dumpvars(0, top_module)`

### Large VCD files
- Use selective dumping with `$dumpvars(0, specific_module)`
- Use `$dumpoff/$dumpon` to skip uninteresting periods
- Consider FST format with Verilator for large designs
