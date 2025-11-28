# VHDL Counter Example

An 8-bit counter demonstrating GHDL simulation with GHW waveform output for NovyWave.

## Files

| File | Description |
|------|-------------|
| `counter.vhd` | 8-bit counter entity and architecture |
| `counter_tb.vhd` | Testbench with stimulus |
| `Makefile` | Build automation |

## Requirements

- [GHDL](https://ghdl.github.io/ghdl/) - VHDL simulator

### Installation

**Ubuntu/Debian:**
```bash
sudo apt install ghdl
```

**macOS:**
```bash
brew install ghdl
```

## Building

Generate the waveform:

```bash
make
```

This will:
1. Analyze the VHDL source files
2. Elaborate the testbench
3. Run simulation
4. Generate `counter.ghw`

## Viewing in NovyWave

Open the generated waveform:

```bash
novywave counter.ghw
```

Or use the Load Files dialog in NovyWave to browse to `counter.ghw`.

## Design Overview

### Counter Entity

```vhdl
entity counter is
    port (
        clk     : in  std_logic;        -- 100 MHz clock
        reset   : in  std_logic;        -- Synchronous reset
        enable  : in  std_logic;        -- Count enable
        count   : out std_logic_vector(7 downto 0);  -- 8-bit count
        overflow: out std_logic         -- Overflow indicator
    );
end entity;
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

## Cleaning Up

```bash
make clean
```
