# Amaranth Counter Example

An 8-bit counter implemented in [Amaranth HDL](https://amaranth-lang.org/), demonstrating waveform generation for NovyWave.

## Features

- 8-bit synchronous counter
- Enable control
- Overflow detection
- VCD waveform output for NovyWave
- Optional Verilog RTL generation

## Requirements

### Python 3.8+

Amaranth requires Python 3.8 or later:

```bash
# Check Python version
python3 --version

# Ubuntu/Debian (usually pre-installed)
sudo apt install python3 python3-pip

# Fedora
sudo dnf install python3 python3-pip

# macOS (using Homebrew)
brew install python3
```

### Amaranth

Install using pip:

```bash
# Using make
make setup

# Or manually
pip3 install amaranth
```

## Quick Start

```bash
# Install dependencies (first time only)
make setup

# Generate waveform
make

# Open in NovyWave
novywave counter.vcd
```

## Usage

### Run Simulation

```bash
make sim
# or
python3 counter.py
```

This runs the Amaranth simulation and generates `counter.vcd`.

### Generate Verilog RTL

```bash
make verilog
# or
python3 counter.py verilog
```

This generates `counter.v` synthesizable Verilog.

### Clean Generated Files

```bash
make clean
```

## Project Structure

```
counter/
├── counter.py          # Counter design and simulation
├── requirements.txt    # Python dependencies
├── Makefile           # Build automation
└── README.md          # This file
```

## Design Overview

### Counter Module

The counter has the following interface:

| Signal   | Direction | Width | Description                    |
|----------|-----------|-------|--------------------------------|
| clk      | input     | 1     | Clock (100 MHz)                |
| rst      | input     | 1     | Synchronous reset              |
| enable   | input     | 1     | Count enable                   |
| count    | output    | 8     | Current count value            |
| overflow | output    | 1     | High when count wraps to 0     |

### Simulation Testbench

The simulation demonstrates:

1. **Enable counting** - Start counting from 0
2. **Disable counting** - Pause the counter
3. **Resume counting** - Continue from paused value
4. **Overflow detection** - Count until wrap-around

## Amaranth Code Explained

```python
from amaranth import *

class Counter(Elaboratable):
    def __init__(self):
        # Define ports
        self.enable = Signal(name="enable")
        self.count = Signal(8, name="count")
        self.overflow = Signal(name="overflow")

    def elaborate(self, platform):
        m = Module()

        # Counter increments when enabled
        with m.If(self.enable):
            m.d.sync += self.count.eq(self.count + 1)

        # Overflow when count wraps from 255 to 0
        with m.If(self.enable & (self.count == 255)):
            m.d.sync += self.overflow.eq(1)
        with m.Else():
            m.d.sync += self.overflow.eq(0)

        return m
```

### Key Amaranth Concepts

- **`Signal`**: Hardware signal (wire or register)
- **`m.d.sync`**: Synchronous (clocked) domain assignments
- **`m.d.comb`**: Combinational assignments
- **`with m.If/Elif/Else`**: Conditional logic (like Verilog if/else)

## Amaranth Advantages

- **Pure Python**: No special syntax to learn, just Python
- **Modern design**: Clean, readable code structure
- **Built-in simulation**: No external simulator needed for basic tests
- **Platform support**: Built-in definitions for many FPGA boards
- **Active development**: Well-maintained with growing ecosystem

## Troubleshooting

### "ModuleNotFoundError: No module named 'amaranth'"

Install Amaranth:
```bash
pip3 install amaranth
# or
make setup
```

### "Python version X.X is not supported"

Amaranth requires Python 3.8+. Check your version:
```bash
python3 --version
```

### Using a Virtual Environment

For isolated installation:
```bash
python3 -m venv venv
source venv/bin/activate  # Linux/macOS
# or: venv\Scripts\activate  # Windows
pip install -r requirements.txt
make
```

## Learn More

- [Amaranth Documentation](https://amaranth-lang.org/docs/amaranth/)
- [Amaranth GitHub](https://github.com/amaranth-lang/amaranth)
- [Glasgow Interface Explorer](https://github.com/GlasgowEmbedded/glasgow) - Built with Amaranth
- [NovyWave User Guide](../../../docs/src/user-guide/quick-start.md)
