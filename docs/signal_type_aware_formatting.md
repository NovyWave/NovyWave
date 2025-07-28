# Signal Type Aware Formatting

## Overview

NovyWave automatically selects appropriate default formatting for variables based on their signal type and bit width, while allowing users to override with custom format selections.

## Signal Type Detection

Signal types are determined from waveform files using the wellen library:

- **Binary Signals**: Single or multi-bit binary values (0, 1, X, Z states)
- **Four-Value Signals**: Extended 4-state logic (0, 1, X, Z)
- **Nine-Value Signals**: Full 9-state logic representation
- **String Signals**: Text/string data
- **Real Signals**: Floating-point values

## Default Format Selection Rules

### Binary Signals
- **1-bit signals**: Default to **Binary** format (`"1"`, `"0"`, `"X"`, `"Z"`)
- **2-8 bit signals**: Default to **Hexadecimal** format (`"A"`, `"3F"`)
- **9+ bit signals**: Default to **Hexadecimal** format with grouping

### Multi-Value Signals (Four/Nine-Value)
- **1-bit**: Default to **Binary** format showing state characters
- **Multi-bit**: Default to **Hexadecimal** with X/Z state preservation

### String Signals
- Default to **ASCII/Text** format (no conversion)
- Other formats disabled (shown but grayed out)

### Real Signals
- Default to **Decimal** format with appropriate precision
- **Binary/Hex** formats disabled for floating-point

## Available Format Types

Based on FastWave2.0's VarFormat system:

1. **ASCII/Text** - Text representation of values
2. **Binary** - Raw binary (`"10110011"`)
3. **Binary (Grouped)** - 4-bit grouped (`"1011 0011"`)
4. **Hexadecimal** - Hex representation (most common default)
5. **Octal** - Octal representation
6. **Signed Integer** - Two's complement signed
7. **Unsigned Integer** - Unsigned decimal

## Format Availability Matrix

| Signal Type | ASCII | Binary | Bin (Grouped) | Hex | Octal | Signed | Unsigned |
|-------------|-------|--------|---------------|-----|-------|--------|----------|
| 1-bit Binary | ❌ | ✅ (default) | ❌ | ✅ | ✅ | ❌ | ✅ |
| Multi-bit Binary | ✅ | ✅ | ✅ | ✅ (default) | ✅ | ✅ | ✅ |
| String | ✅ (default) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Real | ✅ (default) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

✅ = Available, ❌ = Disabled (grayed out)

## User Interaction

- **Default**: System automatically selects appropriate format based on signal type
- **Override**: User can select any available format from dropdown
- **Persistence**: Format choices saved per-variable in `.novywave` config files
- **Dynamic**: Format requests sent on-demand when user changes selection or timeline position changes

## Implementation Notes

- Format selection persists across sessions via `SelectedVariable.formatter` field
- On-demand backend queries prevent unnecessary computation
- Disabled formats shown in dropdown but non-selectable for clear UX
- Compact display format: `"15 (Signed)"`, `"F (Hex)"` for dropdown options