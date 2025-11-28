# Spade Counter Example

An 8-bit counter implemented in [Spade HDL](https://spade-lang.org/), demonstrating waveform generation for NovyWave.

## What is Spade?

Spade is a hardware description language inspired by Rust, with strong typing, pattern matching, and functional programming features. It compiles to Verilog for synthesis and simulation.

## Requirements

- **Spade compiler**: `cargo install spade-lang`
- **Icarus Verilog**: For simulation (`iverilog`, `vvp`)

### Installing Spade

```bash
# Requires Rust toolchain
cargo install spade-lang

# Verify installation
spade --version
```

### Installing Icarus Verilog

```bash
# Ubuntu/Debian
sudo apt install iverilog

# Fedora
sudo dnf install iverilog

# macOS
brew install icarus-verilog
```

## Files

| File | Description |
|------|-------------|
| `counter.spade` | Spade counter design with enable and overflow |
| `counter_tb.v` | Verilog testbench for simulation |
| `Makefile` | Build automation |

## Quick Start

```bash
# Compile and simulate
make

# View in NovyWave
novywave counter.vcd
```

## Build Steps

1. **Compile Spade to Verilog**:
   ```bash
   spade counter.spade -o counter_gen.v
   ```

2. **Compile with Icarus Verilog** (SystemVerilog mode required):
   ```bash
   iverilog -g2012 -o counter_sim counter_gen.v counter_tb.v
   ```

3. **Run simulation**:
   ```bash
   vvp counter_sim
   ```

4. **View waveform**:
   ```bash
   novywave counter.vcd
   ```

## Design Overview

### Counter Entity

```spade
entity counter(clk: clock, rst: bool, enable: bool) -> (uint<8>, bool) {
    reg(clk) count: uint<8> reset(rst: 0) = if enable {
        trunc(count + 1)
    } else {
        count
    };

    let overflow = enable && (count == 255);
    (count, overflow)
}
```

**Features:**
- Synchronous reset
- Enable control
- 8-bit counter with overflow detection
- Functional register update expressions

### Spade Language Highlights

- **Strong typing**: `uint<8>` is an 8-bit unsigned integer
- **Functional style**: Registers defined with update expressions
- **Pattern matching**: Not shown here, but powerful for state machines
- **No implicit truncation**: Must use `trunc()` explicitly

## Testbench

The Verilog testbench (`counter_tb.v`) exercises:
1. Reset behavior
2. Enable/disable counting
3. Counting to overflow (255 → 0)
4. Reset during operation

## Expected Waveform

When viewed in NovyWave, you'll see:
- `clk`: 100 MHz clock (10ns period)
- `rst`: Active during first 100ns
- `enable`: Toggled during tests
- `count[7:0]`: Incrementing counter value
- `overflow`: Pulse when count wraps from 255

## Clean Up

```bash
make clean
```

## Learn More

- [Spade Language Documentation](https://spade-lang.org/docs/)
- [Spade GitHub Repository](https://gitlab.com/spade-lang/spade)
- [Spade by Example](https://spade-lang.org/docs/intro.html)
