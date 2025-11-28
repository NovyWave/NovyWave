# NovyWave HDL Examples

Example hardware design projects demonstrating waveform generation for use with NovyWave.

## Overview

This directory contains example projects in various HDL languages and frameworks that generate waveform files you can view in NovyWave.

| Directory | Language/Framework | Simulator | Output Format |
|-----------|-------------------|-----------|---------------|
| `vhdl/counter` | VHDL | GHDL | GHW |
| `verilog/counter` | Verilog | Icarus Verilog | VCD |
| `spinalhdl/counter` | SpinalHDL (Scala) | Verilator | VCD |
| `amaranth/counter` | Amaranth (Python) | Built-in | VCD |
| `spade/counter` | Spade (Rust-like) | Icarus Verilog | VCD |

## Quick Start

### Traditional HDLs

#### VHDL with GHDL

```bash
# Install GHDL (Ubuntu/Debian)
sudo apt install ghdl

# Generate waveform
cd vhdl/counter
make

# Open in NovyWave
novywave counter.ghw
```

#### Verilog with Icarus Verilog

```bash
# Install Icarus Verilog (Ubuntu/Debian)
sudo apt install iverilog

# Generate waveform
cd verilog/counter
make

# Open in NovyWave
novywave counter.vcd
```

### Modern HDL Frameworks

#### SpinalHDL (Scala-based)

```bash
# Install requirements (Ubuntu/Debian)
sudo apt install openjdk-11-jdk verilator
# Install sbt: https://www.scala-sbt.org/download.html

# Generate waveform
cd spinalhdl/counter
make

# Open in NovyWave
novywave counter.vcd
```

#### Amaranth (Python-based)

```bash
# Install Amaranth
pip3 install amaranth

# Generate waveform
cd amaranth/counter
make

# Open in NovyWave
novywave counter.vcd
```

#### Spade (Rust-inspired HDL)

```bash
# Install Spade (requires Rust toolchain)
cargo install spade-lang

# Generate waveform
cd spade/counter
make

# Open in NovyWave
novywave counter.vcd
```

## Requirements Summary

| Example | Requirements |
|---------|-------------|
| VHDL | GHDL |
| Verilog | Icarus Verilog |
| SpinalHDL | Java 11+, sbt, Verilator |
| Amaranth | Python 3.8+, pip |
| Spade | Rust toolchain, Icarus Verilog |

### All-in-One: OSS CAD Suite

For the easiest setup, install [OSS CAD Suite](https://github.com/YosysHQ/oss-cad-suite-build) which includes GHDL, Icarus Verilog, Verilator, and many other tools:

```bash
# Download from: https://github.com/YosysHQ/oss-cad-suite-build/releases

# Extract and source environment
tar -xzf oss-cad-suite-linux-x64-*.tgz
source oss-cad-suite/environment

# Now you can run VHDL, Verilog, and SpinalHDL examples
```

### Platform-Specific Installation

<details>
<summary>Ubuntu/Debian</summary>

```bash
# Traditional HDL tools
sudo apt install ghdl iverilog

# SpinalHDL requirements
sudo apt install openjdk-11-jdk verilator

# For sbt, follow: https://www.scala-sbt.org/download.html

# Python (usually pre-installed)
sudo apt install python3 python3-pip
pip3 install amaranth
```
</details>

<details>
<summary>Fedora</summary>

```bash
# Traditional HDL tools
sudo dnf install ghdl iverilog

# SpinalHDL requirements
sudo dnf install java-11-openjdk-devel verilator sbt

# Python
sudo dnf install python3 python3-pip
pip3 install amaranth
```
</details>

<details>
<summary>Arch Linux</summary>

```bash
# Traditional HDL tools
sudo pacman -S ghdl iverilog

# SpinalHDL requirements
sudo pacman -S jdk11-openjdk verilator sbt

# Python
sudo pacman -S python python-pip
pip install amaranth
```
</details>

<details>
<summary>macOS (Homebrew)</summary>

```bash
# Traditional HDL tools
brew install ghdl icarus-verilog

# SpinalHDL requirements
brew install openjdk@11 verilator sbt

# Python
brew install python3
pip3 install amaranth
```
</details>

## Example Projects

### Counter (8-bit)

All examples implement the same 8-bit counter design with:
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

This allows you to compare the same design across different HDL languages and see how each generates waveforms.

## Choosing an HDL

| If you want... | Use |
|---------------|-----|
| Industry standard, verbose but explicit | **VHDL** |
| Industry standard, concise | **Verilog** |
| Strong typing, Scala ecosystem | **SpinalHDL** |
| Python familiarity, quick prototyping | **Amaranth** |

### Traditional HDLs (VHDL/Verilog)

**Pros:**
- Industry standard, required for professional work
- Extensive tool support
- Large community and resources

**Cons:**
- Verbose syntax
- Limited abstraction capabilities
- Testbench writing can be tedious

### SpinalHDL (Scala)

**Pros:**
- Powerful type system catches errors at compile time
- Excellent for complex, parameterized designs
- Can generate both Verilog and VHDL
- Great for building reusable IP

**Cons:**
- Requires JVM and sbt
- Steeper learning curve
- Smaller community than traditional HDLs

### Amaranth (Python)

**Pros:**
- Just Python - no new syntax to learn
- Quick to prototype
- Built-in simulation
- Active development

**Cons:**
- Less mature than traditional HDLs
- Smaller ecosystem
- Python performance for large simulations

## Creating Your Own Examples

### General Pattern

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

   **SpinalHDL:**
   ```scala
   SimConfig.withWave.compile(MyDesign()).doSim { dut =>
       // Testbench code
   }
   ```

   **Amaranth:**
   ```python
   with sim.write_vcd("output.vcd"):
       sim.run()
   ```

3. Run simulation to generate waveform file
4. Open the waveform file in NovyWave

## Docker Support

For CI/CD or isolated environments, you can use Docker containers:

```bash
# GHDL/Icarus Verilog
docker run -v $(pwd):/work -w /work hdlc/sim:osvb ghdl -a counter.vhd

# Full OSS CAD Suite
docker run -v $(pwd):/work -w /work hdlc/impl:generic make
```

See the [hdl-containers](https://hdl.github.io/containers/) project for available images.

## Learn More

- [GHDL Documentation](https://ghdl.github.io/ghdl/)
- [Icarus Verilog Documentation](http://iverilog.icarus.com/)
- [SpinalHDL Documentation](https://spinalhdl.github.io/SpinalDoc-RTD/)
- [Amaranth Documentation](https://amaranth-lang.org/docs/amaranth/)
- [NovyWave User Guide](../docs/src/user-guide/quick-start.md)
