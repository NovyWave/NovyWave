# SpinalHDL Counter Example

An 8-bit counter implemented in [SpinalHDL](https://spinalhdl.github.io/SpinalDoc-RTD/), demonstrating waveform generation for NovyWave.

## Features

- 8-bit synchronous counter
- Enable control
- Overflow detection
- Synchronous reset
- VCD waveform output for NovyWave

## Requirements

### Java 11+

SpinalHDL requires Java 11 or later:

```bash
# Ubuntu/Debian
sudo apt install openjdk-11-jdk

# Fedora
sudo dnf install java-11-openjdk-devel

# macOS
brew install openjdk@11

# Verify installation
java -version
```

### sbt (Scala Build Tool)

```bash
# Ubuntu/Debian
echo "deb https://repo.scala-sbt.org/scalasbt/debian all main" | sudo tee /etc/apt/sources.list.d/sbt.list
curl -sL "https://keyserver.ubuntu.com/pks/lookup?op=get&search=0x2EE0EA64E40A89B84B2DF73499E82A75642AC823" | sudo apt-key add
sudo apt update
sudo apt install sbt

# Fedora
sudo dnf install sbt

# macOS
brew install sbt

# Verify installation
sbt --version
```

### Verilator (for simulation)

SpinalHDL uses Verilator for simulation:

```bash
# Ubuntu/Debian
sudo apt install verilator

# Fedora
sudo dnf install verilator

# macOS
brew install verilator

# Verify installation
verilator --version
```

## Quick Start

```bash
# Generate waveform (runs simulation)
make

# Open in NovyWave
novywave counter.vcd
```

## Usage

### Run Simulation

```bash
make sim
```

This compiles the SpinalHDL design, runs simulation with Verilator, and generates `counter.vcd`.

### Generate RTL

```bash
# Generate Verilog
make verilog

# Generate VHDL
make vhdl
```

Generated files appear in the `rtl/` directory.

### Clean Build Artifacts

```bash
make clean
```

## Project Structure

```
counter/
├── build.sbt                 # sbt build configuration
├── project/
│   └── build.properties      # sbt version
├── src/main/scala/counter/
│   ├── Counter.scala         # Counter design
│   └── CounterSim.scala      # Simulation testbench
├── Makefile                  # Build automation
└── README.md                 # This file
```

## Design Overview

### Counter Module

The counter has the following interface:

| Port     | Direction | Width | Description                    |
|----------|-----------|-------|--------------------------------|
| clk      | input     | 1     | Clock (active rising edge)     |
| reset    | input     | 1     | Synchronous reset              |
| enable   | input     | 1     | Count enable                   |
| count    | output    | 8     | Current count value            |
| overflow | output    | 1     | High when count wraps to 0     |

### Simulation Testbench

The simulation demonstrates:

1. **Enable counting** - Start counting from 0
2. **Disable counting** - Pause the counter
3. **Resume counting** - Continue from paused value
4. **Overflow detection** - Count until wrap-around
5. **Reset during operation** - Reset while counting

## SpinalHDL Advantages

- **Type-safe**: Catches width mismatches at compile time
- **Powerful abstractions**: Generic components, automatic pipelining
- **Dual output**: Generate both Verilog and VHDL
- **Native simulation**: Fast simulation with Verilator integration
- **Active community**: Well-maintained with good documentation

## Troubleshooting

### "sbt not found"

Install sbt following the instructions above for your platform.

### "java.lang.UnsupportedClassVersionError"

You need Java 11 or later. Check with `java -version` and install a newer JDK if needed.

### "verilator not found"

Install Verilator for simulation support. Alternatively, you can still generate RTL without Verilator.

### Slow first build

The first build downloads dependencies and may take several minutes. Subsequent builds are much faster.

## Learn More

- [SpinalHDL Documentation](https://spinalhdl.github.io/SpinalDoc-RTD/)
- [SpinalHDL GitHub](https://github.com/SpinalHDL/SpinalHDL)
- [VexRiscv](https://github.com/SpinalHDL/VexRiscv) - RISC-V CPU in SpinalHDL
- [NovyWave User Guide](../../../docs/src/user-guide/quick-start.md)
