# Multi-File Projects

NovyWave excels at working with multiple waveform files simultaneously, a common need in complex verification environments.

## Use Cases

### Design Partitioning
When a large design is simulated in parts:
```
cpu_simulation.vcd      # CPU subsystem simulation
memory_simulation.vcd   # Memory controller simulation
io_simulation.vcd       # I/O interface simulation
```

### Regression Testing
Comparing waveforms from different test runs:
```
test_pass.fst           # Known-good simulation
test_current.fst        # Current run to verify
```

### Mixed Language
Combining Verilog and VHDL simulations:
```
verilog_top.vcd         # Verilog wrapper
vhdl_core.ghw           # VHDL implementation
```

## Loading Multiple Files

### Method 1: Multi-Select in Dialog
1. Click **Load Files**
2. Navigate to directory
3. Hold `Ctrl` (or `Cmd` on macOS)
4. Click multiple files
5. Click **Load**

### Method 2: Sequential Loading
1. Load first file
2. Click **Load Files** again
3. Select additional files
4. Previously loaded files remain

### Method 3: Drag and Drop
- Drag multiple files at once from file manager
- Each file is added to the existing file list

## Time Alignment

### Common Zero Point
Most simulations start at time 0, so multiple files naturally align.

### Different Time Spans
When files have different durations:
```
File A: 0 - 100ns
File B: 0 - 1ms
Combined view: 0 - 1ms
```

NovyWave displays the full combined range. Signals from shorter files show as "N/A" outside their time range.

### Handling Misaligned Files
For files with different start times:
```
File A: 0 - 100ns
File B: 50ns - 200ns
```

The timeline adapts to show the full range. Use the cursor to inspect values at specific times.

## Selecting Variables Across Files

Variables from different files appear together in the waveform view:

1. Select scope from File A
2. Add signals to Selected Variables
3. Select scope from File B
4. Add additional signals
5. All signals display on the same timeline

### Variable Identification
Each variable shows its full path for clarity:
```
design.vcd|TOP|cpu|clk
memory.vcd|TOP|mem|clk
```

## Tips for Multi-File Work

### Use Smart Labeling
NovyWave automatically adds path prefixes when files have the same name:
```
module_a/test.vcd  →  module_a/test.vcd
module_b/test.vcd  →  module_b/test.vcd
```

### Start with Zoom Center at 0
Press `Z` to ensure zoom operations align files at time 0.

### Use Full Reset
Press `R` to see the complete combined timeline.

## Configuration Persistence

NovyWave remembers your multi-file setup:
- All loaded files
- Selected scopes
- Selected variables
- Expansion states

Simply reopen NovyWave to resume where you left off.
