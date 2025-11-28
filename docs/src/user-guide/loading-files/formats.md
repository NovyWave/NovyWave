# Supported Waveform Formats

NovyWave supports the most common waveform file formats used in digital design verification.

## VCD - Value Change Dump

**Extension:** `.vcd`

VCD is the most universal waveform format, supported by virtually all Verilog and SystemVerilog simulators.

### Characteristics
- **ASCII format** - Human-readable, can be edited
- **Large file sizes** - No compression
- **Universal support** - Works with any simulator
- **Standard format** - IEEE 1364 specification

### Creating VCD Files

**Verilog (Icarus Verilog):**
```verilog
initial begin
    $dumpfile("output.vcd");
    $dumpvars(0, top_module);
end
```

**Verilator:**
```cpp
// In C++ testbench
Verilated::traceEverOn(true);
VerilatedVcdC* tfp = new VerilatedVcdC;
top->trace(tfp, 99);
tfp->open("output.vcd");
```

### Limitations
- Large file sizes for long simulations
- Slower to parse than binary formats
- No built-in compression

## FST - Fast Signal Trace

**Extension:** `.fst`

FST is a binary format optimized for fast access and small file sizes.

### Characteristics
- **Binary format** - Compressed, not human-readable
- **Small file sizes** - 10-100x smaller than VCD
- **Fast access** - Optimized for random access
- **GTKWave native** - Designed for GTKWave

### Creating FST Files

**Verilator (recommended):**
```cpp
Verilated::traceEverOn(true);
VerilatedFstC* tfp = new VerilatedFstC;
top->trace(tfp, 99);
tfp->open("output.fst");
```

**GTKWave conversion:**
```bash
vcd2fst input.vcd output.fst
```

### Advantages
- 10-100x smaller than equivalent VCD
- Faster loading for large files
- Efficient random access for navigation

## GHW - GHDL Waveform

**Extension:** `.ghw`

GHW is the native waveform format for GHDL, the open-source VHDL simulator.

### Characteristics
- **Binary format** - Compact representation
- **VHDL-specific** - Full VHDL type support
- **Hierarchical** - Preserves VHDL design hierarchy

### Creating GHW Files

**GHDL simulation:**
```bash
# Analyze VHDL files
ghdl -a design.vhd
ghdl -a testbench.vhd

# Elaborate
ghdl -e testbench

# Run with waveform output
ghdl -r testbench --wave=output.ghw
```

### VHDL Type Support
GHW preserves VHDL-specific types:
- `std_logic` and `std_logic_vector`
- Enumerated types
- Records and arrays
- User-defined types

## Format Comparison

| Feature | VCD | FST | GHW |
|---------|-----|-----|-----|
| File Size | Large | Small | Medium |
| Parse Speed | Slow | Fast | Medium |
| Human Readable | Yes | No | No |
| Compression | None | Built-in | Built-in |
| Random Access | No | Yes | Yes |
| Primary Use | Universal | Verilator | GHDL |

## Recommendations

- **Use FST** for Verilator projects (best performance)
- **Use GHW** for GHDL/VHDL projects
- **Use VCD** for maximum compatibility or debugging
