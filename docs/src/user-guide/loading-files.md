# Loading Waveform Files

NovyWave supports loading multiple waveform files simultaneously, making it easy to correlate signals from different simulations or design partitions.

## Loading Files

### Using the File Dialog

1. Click **Load Files** in the Files & Scopes panel header
2. Navigate to your waveform files
3. Select one or more files (use Ctrl/Cmd+click for multiple selection)
4. Click **Load** or press Enter

### Drag and Drop

Simply drag waveform files from your file manager onto the NovyWave window.

### Supported Formats

- **VCD** (`.vcd`) - Value Change Dump, most universal format
- **FST** (`.fst`) - Fast Signal Trace, optimized for large files
- **GHW** (`.ghw`) - GHDL Waveform, from VHDL simulations

See [Supported Formats](./loading-files/formats.md) for detailed format information.

## Working with Loaded Files

### File Tree Structure

Loaded files appear in the Files & Scopes panel as a tree:

```
📄 design.vcd (0-100ns)
  └── 📁 TOP
      ├── 📁 cpu
      │   └── 📁 alu
      └── 📁 memory
```

Each file shows:
- **File name** (with disambiguation path if needed)
- **Time span** (e.g., 0-100ns)
- **Expandable hierarchy** of modules/scopes

### Selecting Scopes

Click the **checkbox** next to a scope to select it. Only one scope can be selected at a time. The Variables panel shows signals from the selected scope.

### Expanding/Collapsing

- Click the **chevron** (▶/▼) to expand or collapse a scope
- Click anywhere on the row (except checkbox) to toggle expansion
- Expansion state is preserved between sessions

### Removing Files

- Click the **X** button on a file row to remove it
- Use **Remove All** in the header to clear all files

## File Disambiguation

When loading files with the same name from different directories, NovyWave adds path prefixes for clarity:

```
project/module_a/test.vcd  →  module_a/test.vcd
project/module_b/test.vcd  →  module_b/test.vcd
```

The prefix shows just enough path to distinguish files.

## Multi-File Workflows

See [Multi-File Projects](./loading-files/multi-file.md) for advanced multi-file scenarios.

## Troubleshooting

### File Won't Load

- Check the file format is supported (VCD, FST, GHW)
- Ensure the file isn't corrupted
- Check file permissions

### Slow Loading

Large files (>100MB) may take a few seconds to parse. NovyWave shows a loading indicator during parsing.

### Missing Signals

- Expand the file tree to find scopes
- Use the search box in Variables panel to filter
- Check that the correct scope is selected
