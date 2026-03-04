# NovyWave Test Waveform Files

This directory contains various test waveform files for testing NovyWave's capabilities, edge cases, and performance.

## Files Created

### 1. `complex.vcd` (300 seconds, 1s timescale)
- **Compatible with `simple.vcd`** - same timescale for simultaneous viewing
- Multiple modules with deep hierarchy (cpu, memory, decoder, debug)
- Various bit widths: 1-bit to 512-bit signals
- Edge cases:
  - All zeros, all ones, alternating patterns
  - Very wide buses (128-bit, 256-bit, 512-bit)
  - Rapid value changes between #25-#30 for performance testing
  - State machines and counters
  - Reset sequences and error flags

### 2. `stress_test.vcd` (300μs, 1ns timescale)
- **Performance stress testing** with rapid transitions
- Edge cases:
  - X (unknown) and Z (high-impedance) states
  - Mixed state values (combination of 0, 1, X, Z)
  - Ultra-rapid toggling (50 transitions at #50000 and #100000)
  - Very long bus values (1024-bit and 2048-bit)
  - ASCII text in signals (simulating string data)
  - Extremely rapid sequential changes for zoom/pan testing

### 3. `generate_test_waveforms.py`
Python script to generate additional test files:
- **extreme_test.vcd**: Ultra-fast signals (10GHz clock), 4096-bit buses, 100+ signals, deep hierarchy
- **protocol_test.vcd**: Realistic protocol simulations (I2C, SPI, UART, AXI)
- **performance_test.vcd**: 500 signals, 100,000 time points for performance benchmarking

## Usage

### Manual Test Files
```bash
# Use the pre-generated files
novywave complex.vcd
novywave stress_test.vcd

# Load multiple files together
novywave simple.vcd complex.vcd
```

### Generate Additional Test Files
```bash
# Run the Python generator
./generate_test_waveforms.py

# This creates:
# - extreme_test.vcd
# - protocol_test.vcd  
# - performance_test.vcd
```

### Convert to FST Format
If you have GTKWave installed:
```bash
vcd2fst complex.vcd complex.fst
vcd2fst stress_test.vcd stress_test.fst
```

## Test Scenarios

### 1. Multi-file Loading
Load `simple.vcd` and `complex.vcd` together - they have compatible timescales

### 2. Performance Testing
- Load `stress_test.vcd` and zoom to #50000-#100000 for rapid toggle testing
- Load `performance_test.vcd` for testing with 500 signals

### 3. Edge Case Testing
- X and Z states: Check signals in `stress_test.vcd` module "edge_cases"
- Wide buses: Check 512-bit, 1024-bit, 2048-bit signals
- Deep hierarchy: Navigate through 5+ levels in `extreme_test.vcd`

### 4. UI Stress Testing
- Long signal names in `extreme_test.vcd`
- Many signals (100+) in single scope
- Rapid zoom/pan with high-frequency signals

## Known Edge Cases Covered

1. **Value Types**: Binary, X (unknown), Z (high-impedance), mixed states
2. **Bus Widths**: 1-bit to 4096-bit
3. **Timing**: Picosecond to second timescales
4. **Hierarchy**: Flat to 5+ levels deep
5. **Performance**: Up to 100 rapid transitions per signal
6. **Names**: Very long names, special characters, array notation
7. **Data Patterns**: All 0s, all 1s, alternating, random, incremental
8. **Protocol Patterns**: Start/stop conditions, bursts, transactions

## Notes

- FST files are more efficient for large waveforms but require conversion
- The Python generator can create custom test patterns on demand
- Test files use realistic patterns that might be seen in actual designs