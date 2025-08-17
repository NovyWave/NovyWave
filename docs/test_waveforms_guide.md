# NovyWave Test Waveforms Guide

## Overview

Comprehensive test waveform files for validating NovyWave's rendering, performance, and edge case handling capabilities.

## Test Files

### Basic Compatibility Test
- **simple.vcd** + **complex.vcd** - Load together, same 1s timescale for synchronized viewing
- Tests multi-file loading and time alignment

### Performance Stress Tests

#### complex.vcd (300 seconds, 1s timescale)
- 24 signals across 4 modules (cpu, memory, decoder, debug)
- Signal widths: 1, 4, 7, 8, 12, 16, 20, 24, 32, 48, 64, 128, 256, 512 bits
- **Performance hotspot**: #25-#30 with rapid successive transitions
- **Pattern variety**: All-zeros, all-ones, alternating (0xAA/0x55), counting sequences
- Tests rendering of very wide buses (512-bit massive_debug signal)

#### stress_test.vcd (300μs, 1ns timescale)
- **Ultra-rapid toggling**: 50 transitions at #50000, 100 transitions at #100000
- **Edge cases**: X (unknown) and Z (high-impedance) states
- **Massive buses**: 1024-bit and 2048-bit signals
- **ASCII encoding**: Text data in 64-bit signals
- Tests zoom/pan performance with rapid changes

#### performance_test.vcd (100μs, 1ns timescale)
- **500 signals** across 10 modules
- **100,000 time points**
- 10% of signals change at each time point
- Ultimate rendering performance benchmark

### Edge Case Tests

#### extreme_test.vcd (1ms, 1ps timescale)
- **Ultra-fast signals**: 10GHz clock (toggles every 100ps)
- **Extreme widths**: 4096-bit bus
- **Deep hierarchy**: 5-level nested modules
- **100+ signals** in single module
- **Long signal names**: Tests UI text rendering limits
- Tests picosecond precision and extreme zoom levels

#### protocol_test.vcd (10μs, 1ns timescale)
- **Real protocols**: I2C, SPI, UART, AXI simulations
- **Transaction patterns**: Start/stop conditions, bursts, handshaking
- Tests protocol analysis use cases

## Test Scenarios

### 1. Rendering Performance
```bash
# Load performance test
# Zoom to different levels and measure FPS
# Pan rapidly across time range
```
- Expected: Smooth rendering even with 500 signals

### 2. Zoom Limits
```bash
# Load extreme_test.vcd
# Zoom in to see individual picosecond transitions
# Zoom out to see full millisecond range
```
- Expected: Stable rendering at all zoom levels

### 3. Wide Bus Display
```bash
# Load stress_test.vcd
# Select the 2048-bit signal
# Check value display and formatting
```
- Expected: Proper hex display, no overflow

### 4. Rapid Transitions
```bash
# Load stress_test.vcd
# Navigate to #50000-#100000
# Zoom in to see individual transitions
```
- Expected: All transitions visible, no missing edges

### 5. Multi-File Synchronization
```bash
# Load simple.vcd and complex.vcd together
# Select signals from both files
# Verify time alignment
```
- Expected: Perfect time synchronization

### 6. Unknown States
```bash
# Load stress_test.vcd
# Find signals with X and Z states
# Verify proper rendering (typically red for X, blue for Z)
```
- Expected: Distinct visual representation

## Performance Benchmarks

| Test File | Signals | Time Points | Key Challenge |
|-----------|---------|-------------|---------------|
| complex.vcd | 24 | 300 | Wide buses (512-bit) |
| stress_test.vcd | 12 | 300,000 | Rapid transitions |
| performance_test.vcd | 500 | 100,000 | Many signals |
| extreme_test.vcd | 115 | 10,000 | Deep hierarchy, 4096-bit bus |

## Known Stress Points

1. **Zoom to #50000 in stress_test.vcd** - 100 transitions in microseconds
2. **4096-bit bus in extreme_test.vcd** - Extreme width handling
3. **500 signals in performance_test.vcd** - Vertical scrolling performance
4. **Picosecond timescale in extreme_test.vcd** - Precision limits

## Usage

### Quick Performance Test
```bash
# Start the app
makers start

# In another terminal, monitor performance
tail -f dev_server.log

# Load test files through UI
# Navigate to test_files/ directory
# Select multiple files for stress testing
```

### Manual Testing Checklist
- [ ] Load each test file individually
- [ ] Load simple.vcd + complex.vcd together
- [ ] Zoom in/out to extremes in each file
- [ ] Pan rapidly across time ranges
- [ ] Select very wide buses (512+ bits)
- [ ] Find and verify X/Z state rendering
- [ ] Test with 100+ signals visible
- [ ] Verify no rendering artifacts at zoom extremes