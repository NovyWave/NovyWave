# Working with Multiple Files

This tutorial shows how to load and compare signals from multiple waveform files.

## Scenario

You have two simulation results to compare:
- `baseline.vcd` - Known-good reference simulation
- `current.vcd` - Current design under test

## Step 1: Load Multiple Files

### Method A: Multi-Select

1. Click **Load Files**
2. Navigate to your files directory
3. Hold `Ctrl` (or `Cmd` on macOS)
4. Click `baseline.vcd`
5. Click `current.vcd`
6. Click **Load**

### Method B: Sequential Loading

1. Click **Load Files** → Select `baseline.vcd` → Click **Load**
2. Click **Load Files** → Select `current.vcd` → Click **Load**

Both files now appear in the Files & Scopes panel.

## Step 2: Understand File Disambiguation

If files have the same name from different directories, NovyWave adds path prefixes:

```
tests/pass/design.vcd  →  pass/design.vcd
tests/fail/design.vcd  →  fail/design.vcd
```

## Step 3: Select Signals from Multiple Files

### From First File

1. Expand `baseline.vcd`
2. Navigate to and click checkbox for `TOP > dut`
3. In Variables panel, click `clk` and `data_out`

### From Second File

1. Expand `current.vcd`
2. Navigate to and click checkbox for `TOP > dut`
3. In Variables panel, click `data_out` (clk from first file is already there)

## Step 4: Viewing Combined Signals

The Selected Variables panel now shows signals from both files:

```
baseline.vcd|TOP|dut|clk      [1]   [Bin ▼]   ████████
baseline.vcd|TOP|dut|data_out [0x42][Hex ▼]   ████░░░░
current.vcd|TOP|dut|data_out  [0x42][Hex ▼]   ████░░░░
```

## Step 5: Time Alignment

Both files typically start at time 0, so they align automatically.

Press `R` to see the full combined timeline. The view extends to cover the longest file.

## Step 6: Comparing Signals

### Visual Comparison

Look for differences in the waveform patterns. Mismatches stand out when signals are adjacent.

### Cursor-Based Comparison

1. Press `R` to see full timeline
2. Click on an area that looks different
3. Use `Q`/`E` to fine-tune cursor position
4. Compare values in the Value column

### Jump to Differences

Use `Shift+Q` and `Shift+E` to jump between transitions. If the files differ, one signal will transition while the other doesn't.

## Step 7: Different Time Ranges

If your files have different durations:

```
baseline.vcd: 0 - 100ns
current.vcd:  0 - 150ns
```

- The combined view shows 0-150ns
- `baseline.vcd` signals show `N/A` from 100ns-150ns
- This indicates where the files don't overlap

## Step 8: Organizing Your View

### Reorder Signals

Signals appear in the order you add them. To group related signals:
1. Remove all signals (click X on each)
2. Add signals in desired order

### Use Consistent Formatting

Set the same format (Hex, Bin, etc.) for signals you're comparing to make differences easier to spot.

## Practical Tips

### Compare Clock Signals

Add clock signals from both files to verify time alignment:
- If clocks don't match, there may be time scale differences

### Focus on Outputs

When debugging, compare output signals first:
- Outputs show the final result of internal differences
- Trace backward from output differences to find root cause

### Use Common Ancestor Scope

Select scopes at the same hierarchy level in both files for meaningful comparison.

## Removing Files

To focus on fewer files:
- Click **X** on individual files in Files & Scopes panel
- Or click **Remove All** to clear everything

## Save Your Comparison Setup

NovyWave automatically saves:
- Both loaded files
- All selected signals
- Your comparison view settings

Reopen NovyWave later to continue your analysis.
