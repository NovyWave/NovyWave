# NovyWave Waveform Testing Guide

This document describes how to test that the example waveform files load correctly in NovyWave.

## Prerequisites

1. Generate all example waveforms:
   ```bash
   cd examples
   just sim-all
   ```

2. Validate waveform files (uses same wellen library as NovyWave):
   ```bash
   just validate
   # Or with verbose output:
   just validate-verbose
   ```

3. Start NovyWave:
   ```bash
   cd ..  # Back to NovyWave root
   makers start
   ```

4. Wait for compilation to complete (check `dev_server.log`)

## Test Files

| Example    | Format | File Path                              | Variables | Transitions | Timescale |
|------------|--------|----------------------------------------|-----------|-------------|-----------|
| VHDL       | GHW    | `examples/vhdl/counter/counter.ghw`    | ~10       | ~1500       | ns        |
| Verilog    | VCD    | `examples/verilog/counter/counter.vcd` | 11        | 1,523       | 1ps       |
| SpinalHDL  | VCD    | `examples/spinalhdl/counter/counter.vcd` | 13      | 1,115       | ns        |
| Amaranth   | VCD    | `examples/amaranth/counter/counter.vcd` | 5         | 827         | ns        |
| Spade      | VCD    | `examples/spade/counter/counter.vcd`   | 28        | 2,223       | 1ps       |

### Expected Signals by Example

**VHDL (counter_tb/uut):**
- `clk`, `reset`, `enable`, `count[7:0]`, `overflow`

**Verilog (counter_tb):**
- `clk`, `reset`, `enable`, `count[7:0]`, `overflow`

**SpinalHDL (TOP/Counter):**
- `clk`, `reset`, `io_enable`, `io_count[7:0]`, `io_overflow`

**Amaranth (top/counter):**
- `clk`, `rst`, `enable`, `count[7:0]`, `overflow`

**Spade (TOP):**
- `clk`, `rst`, `enable`, `count[7:0]`, `overflow`

## Manual Testing Checklist

### For each file:

1. **Load File**
   - Click "Load Files" in Files & Scopes panel
   - Navigate to the file location
   - Select and load the file

2. **Verify File Loads**
   - [ ] File appears in Files & Scopes panel with correct name
   - [ ] File shows timespan (e.g., "0-250s" or "0-1000ns")
   - [ ] No error icon or warning displayed

3. **Verify Scopes Parse Correctly**
   - [ ] Expand the file in the tree view
   - [ ] Check that scope hierarchy is visible (e.g., "TOP", "counter", "testbench")
   - [ ] Select a scope to see variables

4. **Verify Signals/Variables**
   - [ ] Variables panel shows signal list after scope selection
   - [ ] Signal types are displayed (Wire 1-bit, Logic 8-bit, etc.)
   - [ ] Signal names match expected signals from table above

5. **Verify Waveform Display**
   - [ ] Select multiple variables to add to Selected Variables panel
   - [ ] Wave column shows transitions with correct timing
   - [ ] Formatted values display correctly (Hex, Bin, etc.)
   - [ ] Timeline shows appropriate time units (ns, μs, ms, s)

6. **Verify Signal Values**
   - [ ] `clk` shows alternating 0/1 pattern
   - [ ] `count` increments from 0 to 255 (or max value)
   - [ ] `overflow` pulses high when count wraps around
   - [ ] Reset behavior visible at start of simulation

## Automated Testing with Claude Code

When running with Claude Code and Browser MCP:

```
# Navigate to NovyWave
mcp__browsermcp__browser_navigate to http://localhost:8080

# Take initial screenshot
mcp__browsermcp__browser_screenshot

# Get accessibility tree to find Load Files button
mcp__browsermcp__browser_snapshot

# Click Load Files button
mcp__browsermcp__browser_click on "Load Files" button

# Navigate to examples folder and load files
# ... continue with file selection

# Verify loaded files appear
mcp__browsermcp__browser_snapshot

# Check console for errors
mcp__browsermcp__browser_get_console_logs
```

## Expected Results

### VCD File Structure (Verilog/SpinalHDL/Amaranth/Spade)
```
$version ... $end
$timescale ... $end
$scope module ... $end
$var wire/reg ... $end
$upscope $end
$enddefinitions $end
#0
$dumpvars
...
#time_value
signal_changes
```

### GHW File (VHDL)
- Binary format specific to GHDL
- Contains type information for VHDL signals
- Preserves VHDL enumeration types

## Troubleshooting

### File Won't Load
- Check file permissions
- Verify file is not corrupted (try opening with gtkwave)
- Check console for parsing errors

### No Signals Shown
- Ensure scope is selected in Files & Scopes panel
- Check that simulation actually wrote data (not empty file)

### Timing Issues
- Verify timescale in VCD header matches expectations
- Check that time values are within expected range
