# NovyWave HDL Examples

Example hardware design projects demonstrating waveform generation for use with NovyWave.

## Overview

This directory contains example projects in both VHDL and Verilog that generate waveform files you can view in NovyWave.

| Directory | Language | Simulator | Output Format |
|-----------|----------|-----------|---------------|
| `vhdl/counter` | VHDL | GHDL | GHW |
| `verilog/counter` | Verilog | Icarus Verilog | VCD |

## Quick Start

### VHDL with GHDL

```bash
# Install GHDL (Ubuntu/Debian)
sudo apt install ghdl

# Generate waveform
cd vhdl/counter
make

# Open in NovyWave
novywave counter.ghw
```

### Verilog with Icarus Verilog

```bash
# Install Icarus Verilog (Ubuntu/Debian)
sudo apt install iverilog

# Generate waveform
cd verilog/counter
make

# Open in NovyWave
novywave counter.vcd
```

## Requirements

### GHDL (for VHDL examples)

- **Ubuntu/Debian**: `sudo apt install ghdl`
- **Fedora**: `sudo dnf install ghdl`
- **Arch Linux**: `sudo pacman -S ghdl`
- **macOS**: `brew install ghdl`

### Icarus Verilog (for Verilog examples)

- **Ubuntu/Debian**: `sudo apt install iverilog`
- **Fedora**: `sudo dnf install iverilog`
- **Arch Linux**: `sudo pacman -S iverilog`
- **macOS**: `brew install icarus-verilog`

## Example Projects

### Counter (8-bit)

A simple 8-bit counter with:
- Clock input (100 MHz)
- Synchronous reset
- Enable control
- 8-bit count output
- Overflow indicator

The testbench demonstrates:
- Reset behavior
- Count enable/disable
- Counting to overflow
- Reset during operation

## Creating Your Own Examples

1. Write your HDL design and testbench
2. Configure waveform dumping:

   **VHDL (GHDL):**
   ```bash
   ghdl -r testbench --wave=output.ghw
   ```

   **Verilog (Icarus):**
   ```verilog
   initial begin
       $dumpfile("output.vcd");
       $dumpvars(0, testbench);
   end
   ```

3. Run simulation to generate waveform file
4. Open the waveform file in NovyWave

## Learn More

- [GHDL Documentation](https://ghdl.github.io/ghdl/)
- [Icarus Verilog Documentation](http://iverilog.icarus.com/)
- [NovyWave User Guide](../docs/src/user-guide/quick-start.md)
