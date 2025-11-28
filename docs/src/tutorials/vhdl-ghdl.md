# VHDL with GHDL

This tutorial shows how to generate waveforms from VHDL using GHDL and view them in NovyWave.

## Prerequisites

- [GHDL](https://ghdl.github.io/ghdl/) installed
- NovyWave installed

### Installing GHDL

**Ubuntu/Debian:**
```bash
sudo apt-get install ghdl
```

**macOS:**
```bash
brew install ghdl
```

**OSS CAD Suite (recommended):**
Download from [YosysHQ/oss-cad-suite-build](https://github.com/YosysHQ/oss-cad-suite-build)

## Step 1: Create a Simple Design

Create `counter.vhd`:

```vhdl
library ieee;
use ieee.std_logic_1164.all;
use ieee.numeric_std.all;

entity counter is
    port (
        clk     : in  std_logic;
        reset   : in  std_logic;
        count   : out std_logic_vector(7 downto 0)
    );
end counter;

architecture rtl of counter is
    signal count_reg : unsigned(7 downto 0) := (others => '0');
begin
    process(clk, reset)
    begin
        if reset = '1' then
            count_reg <= (others => '0');
        elsif rising_edge(clk) then
            count_reg <= count_reg + 1;
        end if;
    end process;

    count <= std_logic_vector(count_reg);
end rtl;
```

## Step 2: Create a Testbench

Create `counter_tb.vhd`:

```vhdl
library ieee;
use ieee.std_logic_1164.all;

entity counter_tb is
end counter_tb;

architecture tb of counter_tb is
    signal clk   : std_logic := '0';
    signal reset : std_logic := '1';
    signal count : std_logic_vector(7 downto 0);

    constant CLK_PERIOD : time := 10 ns;
begin
    -- Instantiate the design
    dut: entity work.counter
        port map (
            clk   => clk,
            reset => reset,
            count => count
        );

    -- Clock generation
    clk_process: process
    begin
        for i in 0 to 299 loop
            clk <= '0';
            wait for CLK_PERIOD / 2;
            clk <= '1';
            wait for CLK_PERIOD / 2;
        end loop;
        wait;
    end process;

    -- Stimulus
    stim_process: process
    begin
        reset <= '1';
        wait for 25 ns;
        reset <= '0';
        wait;
    end process;
end tb;
```

## Step 3: Analyze VHDL Files

```bash
ghdl -a counter.vhd
ghdl -a counter_tb.vhd
```

## Step 4: Elaborate the Testbench

```bash
ghdl -e counter_tb
```

## Step 5: Run with Waveform Output

Generate a GHW waveform file:

```bash
ghdl -r counter_tb --wave=counter.ghw --stop-time=3000ns
```

This creates `counter.ghw` with all signal transitions.

## Step 6: View in NovyWave

1. Open NovyWave
2. Click **Load Files**
3. Select `counter.ghw`
4. Click **Load**

The file appears in Files & Scopes:

```
📄 counter.ghw (0-3μs)
  └── 📁 counter_tb
      └── 📁 dut
```

## Step 7: Select Signals

1. Click checkbox next to `counter_tb`
2. In Variables panel, click:
   - `clk`
   - `reset`
   - `count`

## Step 8: Explore the Waveform

- Press `R` for full view
- Press `W` to zoom into the reset release
- Use `Shift+E` to jump between transitions
- Watch the counter increment every clock cycle

## GHDL Options for Waveforms

### Output Format

```bash
# GHW format (recommended for NovyWave)
ghdl -r testbench --wave=output.ghw

# VCD format (alternative)
ghdl -r testbench --vcd=output.vcd
```

### Simulation Time

```bash
# Run for specific time
ghdl -r testbench --wave=output.ghw --stop-time=1ms

# Run until simulation ends naturally
ghdl -r testbench --wave=output.ghw
```

### Signal Selection

By default, GHDL dumps all signals. For large designs, this can create huge files.

```bash
# Dump only top-level signals
ghdl -r testbench --wave=output.ghw --write-wave-opt=waveopts.txt
```

## Next Steps

- Try modifying the counter to count by 2
- Add more signals to observe
- Create a more complex testbench
- Compare multiple simulation runs using [multi-file tutorial](./multi-file.md)

## Troubleshooting

### "cannot find entity"
- Ensure files are analyzed in dependency order
- Check entity names match between files

### Empty waveform file
- Verify simulation actually runs (check for assertion errors)
- Ensure testbench has signal transitions

### Large file sizes
- Use GHW instead of VCD
- Limit simulation time
- Use signal selection options
