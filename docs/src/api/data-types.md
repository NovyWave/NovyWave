# Data Types

This reference documents the key data types used in NovyWave's internal architecture.

## Waveform Data

### TrackedFile

Represents a loaded waveform file:

```rust
pub struct TrackedFile {
    pub id: String,           // Unique identifier
    pub path: String,         // Absolute file path
    pub filename: String,     // File name only
    pub state: FileState,     // Loading state
}
```

### FileState

The loading state of a waveform file:

```rust
pub enum FileState {
    Loading,                  // Currently parsing
    Loaded(WaveformFile),     // Successfully loaded
    Failed(String),           // Error message
}
```

### WaveformFile

Parsed waveform data:

```rust
pub struct WaveformFile {
    pub path: String,
    pub filename: String,
    pub format: FileFormat,
    pub time_span: (f64, f64),  // (start_ns, end_ns)
    pub time_unit: String,
    pub scopes: Vec<ScopeData>,
}
```

### FileFormat

Supported waveform formats:

```rust
pub enum FileFormat {
    Vcd,   // Value Change Dump
    Fst,   // Fast Signal Trace
    Ghw,   // GHDL Waveform
}
```

## Signal Data

### ScopeData

A scope/module in the hierarchy:

```rust
pub struct ScopeData {
    pub name: String,
    pub full_path: String,        // scope1|scope2|name
    pub signals: Vec<Signal>,
    pub children: Vec<ScopeData>,
}
```

### Signal

A signal definition:

```rust
pub struct Signal {
    pub name: String,
    pub signal_type: SignalType,
    pub bit_width: u32,
}
```

### SignalType

The type of signal:

```rust
pub enum SignalType {
    Wire,
    Reg,
    Logic,
    Integer,
    Real,
    Other(String),
}
```

### SignalTransition

A value change in a signal:

```rust
pub struct SignalTransition {
    pub time_ns: u64,      // Time in nanoseconds
    pub value: String,     // Value as string
}
```

## Selection Data

### SelectedVariable

A variable selected for display:

```rust
pub struct SelectedVariable {
    pub unique_id: String,        // file|scope|name
    pub file_path: String,
    pub scope_path: String,
    pub variable_name: String,
    pub formatter: Formatter,
}
```

### Formatter

Display format for values:

```rust
pub enum Formatter {
    Ascii,       // ASCII text
    Binary,      // 1s and 0s
    BinaryWithGroups, // 1101 0011
    Hexadecimal, // 0xAB
    Octal,       // 0o123
    Signed,      // -42
    Unsigned,    // 42
}
```

## Time Types

### TimeNs

Time in nanoseconds (internal representation):

```rust
pub type TimeNs = u64;  // All time values as nanoseconds
```

### Time Formatting

Time values are formatted based on magnitude:

| Range | Format | Example |
|-------|--------|---------|
| < 1000ns | ns | `125ns` |
| 1μs - 1ms | μs | `125.0μs` |
| 1ms - 1s | ms | `125.0ms` |
| ≥ 1s | s | `125.0s` |

## Configuration Types

### AppConfig

Application configuration structure:

```rust
pub struct AppConfig {
    pub workspace: WorkspaceSection,
    pub files: FilesSection,
    pub scope: ScopeSection,
    pub variables: VariablesSection,
    pub panels: PanelsSection,
    pub timeline: TimelineSection,
}
```

### DockMode

Panel layout mode:

```rust
pub enum DockMode {
    Right,   // Selected Variables on right
    Bottom,  // Selected Variables on bottom
}
```

### Theme

UI theme:

```rust
pub enum Theme {
    Dark,
    Light,
}
```

## Special Values

### Signal States

Special signal values:

| Value | Meaning | Display |
|-------|---------|---------|
| `Z` | High-impedance | `Z` |
| `X` | Unknown | `X` |
| `U` | Uninitialized | `U` |

### N/A State

When cursor is outside file's time range:
- Display: `N/A`
- Indicates no data available at that time
