# Verilog Counter Example

An 8-bit counter demonstrating Icarus Verilog simulation with VCD waveform output for NovyWave.

## Files

| File | Description |
|------|-------------|
| `counter.v` | 8-bit counter module |
| `counter_tb.v` | Testbench with stimulus |
| `Makefile` | Build automation |

## Requirements

- [Icarus Verilog](http://iverilog.icarus.com/) - Verilog simulator

### Installation

**Ubuntu/Debian:**
```bash
sudo apt install iverilog
```

**macOS:**
```bash
brew install icarus-verilog
```

## Building

Generate the waveform:

```bash
make
```

This will:
1. Compile the Verilog source files
2. Run the simulation
3. Generate `counter.vcd`

## Viewing in NovyWave

Open the generated waveform:

```bash
novywave counter.vcd
```

Or use the Load Files dialog in NovyWave to browse to `counter.vcd`.

## Design Overview

### Counter Module

```verilog
module counter (
    input  wire       clk,       // 100 MHz clock
    input  wire       reset,     // Asynchronous reset
    input  wire       enable,    // Count enable
    output reg  [7:0] count,     // 8-bit count
    output reg        overflow   // Overflow indicator
);
```

### Testbench Sequence

1. **Reset** (0-50ns): Assert reset
2. **Count** (50-350ns): Enable counting
3. **Pause** (350-400ns): Disable counting
4. **Resume** (400-600ns): Continue counting
5. **Reset during count** (600-630ns): Test reset behavior
6. **Count to overflow** (630-3830ns): Continue until overflow

## Signals to Observe

- `clk` - Clock signal
- `reset` - Reset control
- `enable` - Count enable
- `count[7:0]` - Counter value (try Hex or Unsigned format)
- `overflow` - Overflow flag

## VCD Dumping

The testbench includes VCD dump commands:

```verilog
initial begin
    $dumpfile("counter.vcd");
    $dumpvars(0, counter_tb);
end
```

## Cleaning Up

```bash
make clean
```
