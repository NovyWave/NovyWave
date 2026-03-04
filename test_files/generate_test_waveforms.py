#!/usr/bin/env python3
"""
Generate complex test waveform files for NovyWave testing.
Supports VCD format and conversion to FST if gtkwave tools are available.

Fixed version that avoids four-state values (x/z) for binary signals.
Only generates '0' and '1' values for proper wellen library compatibility.
"""

import random
import math
import subprocess
import os
from datetime import datetime

class WaveformGenerator:
    def __init__(self, filename, timescale="1ns"):
        self.filename = filename
        self.timescale = timescale
        self.signals = {}
        self.signal_counter = 33  # Start from ASCII '!'
        self.scopes = []
        self.current_scope = []
        
    def add_scope(self, module_name):
        """Add a new scope/module"""
        self.current_scope.append(module_name)
        return self
    
    def exit_scope(self):
        """Exit current scope"""
        if self.current_scope:
            self.current_scope.pop()
        return self
    
    def add_signal(self, name, width, signal_type="wire"):
        """Add a signal to current scope"""
        scope_path = ".".join(self.current_scope)
        signal_id = chr(self.signal_counter)
        self.signal_counter += 1
        if self.signal_counter > 126:  # Use multi-char IDs if needed
            self.signal_counter = 33
            signal_id = "!" + signal_id
            
        self.signals[signal_id] = {
            'name': name,
            'width': width,
            'type': signal_type,
            'scope': scope_path,
            'values': {}
        }
        return signal_id
    
    def set_value(self, signal_id, time, value):
        """Set signal value at specific time"""
        if signal_id in self.signals:
            self.signals[signal_id]['values'][time] = value
    
    def generate_vcd(self):
        """Generate VCD file content"""
        lines = []
        
        # Header
        lines.append(f"$date\n\t{datetime.now().strftime('%c')}\n$end")
        lines.append(f"$version\n\tNovyWave Test Generator 2.0\n$end")
        lines.append(f"$timescale\n\t{self.timescale}\n$end")
        
        # Build scope hierarchy and signals
        current_scope = []
        for signal_id, signal in self.signals.items():
            scope_parts = signal['scope'].split('.') if signal['scope'] else []
            
            # Close scopes that are not common
            while current_scope and (
                len(scope_parts) < len(current_scope) or
                scope_parts[:len(current_scope)] != current_scope
            ):
                lines.append("$upscope $end")
                current_scope.pop()
            
            # Open new scopes
            for i in range(len(current_scope), len(scope_parts)):
                lines.append(f"$scope module {scope_parts[i]} $end")
                current_scope.append(scope_parts[i])
            
            # Add signal
            if signal['width'] == 1:
                lines.append(f"$var {signal['type']} 1 {signal_id} {signal['name']} $end")
            else:
                lines.append(f"$var {signal['type']} {signal['width']} {signal_id} {signal['name']} [{signal['width']-1}:0] $end")
        
        # Close remaining scopes
        while current_scope:
            lines.append("$upscope $end")
            current_scope.pop()
        
        lines.append("$enddefinitions $end")
        
        # Collect all timestamps and sort
        all_times = set()
        for signal in self.signals.values():
            all_times.update(signal['values'].keys())
        
        # Write value changes
        for time in sorted(all_times):
            lines.append(f"#{time}")
            if time == 0:
                lines.append("$dumpvars")
            
            for signal_id, signal in self.signals.items():
                if time in signal['values']:
                    value = signal['values'][time]
                    if signal['width'] == 1:
                        lines.append(f"{value}{signal_id}")
                    else:
                        lines.append(f"b{value} {signal_id}")
            
            if time == 0:
                lines.append("$end")
        
        return "\n".join(lines)
    
    def save(self):
        """Save to VCD file"""
        with open(self.filename, 'w') as f:
            f.write(self.generate_vcd())
        print(f"Generated {self.filename}")
        
        # Try to convert to FST if vcd2fst is available
        fst_filename = self.filename.replace('.vcd', '.fst')
        try:
            subprocess.run(['vcd2fst', self.filename, fst_filename], 
                         capture_output=True, check=True)
            print(f"Converted to {fst_filename}")
        except (subprocess.CalledProcessError, FileNotFoundError):
            print("Note: vcd2fst not found. Install gtkwave to generate FST files.")

def generate_extreme_test():
    """Generate a waveform with extreme edge cases"""
    gen = WaveformGenerator("extreme_test.vcd", "1ps")
    
    # Add various test modules
    gen.add_scope("extreme_test")
    
    # Module 1: Ultra-fast toggling
    gen.add_scope("ultra_fast")
    fast_clk = gen.add_signal("clk_10ghz", 1)
    fast_data = gen.add_signal("data_burst", 32)
    gen.exit_scope()
    
    # Module 2: Very wide buses
    gen.add_scope("wide_buses")
    wide_1k = gen.add_signal("bus_1024bit", 1024)
    wide_4k = gen.add_signal("bus_4096bit", 4096)
    gen.exit_scope()
    
    # Module 3: Deep hierarchy
    gen.add_scope("level1")
    gen.add_scope("level2")
    gen.add_scope("level3")
    gen.add_scope("level4")
    gen.add_scope("level5")
    deep_signal = gen.add_signal("deeply_nested", 8)
    gen.exit_scope()
    gen.exit_scope()
    gen.exit_scope()
    gen.exit_scope()
    gen.exit_scope()
    
    # Module 4: Many signals
    gen.add_scope("many_signals")
    many_sigs = []
    for i in range(100):
        sig = gen.add_signal(f"sig_{i:03d}", random.choice([1, 4, 8, 16, 32]))
        many_sigs.append(sig)
    gen.exit_scope()
    
    # Module 5: Unicode and special names (if supported)
    gen.add_scope("special_names")
    special1 = gen.add_signal("signal_with_very_long_name_that_might_cause_display_issues_in_ui", 16)
    special2 = gen.add_signal("sig[0][1][2]", 8)  # Array notation
    special3 = gen.add_signal("module.subsignal", 4)  # Hierarchical notation
    gen.exit_scope()
    
    gen.exit_scope()  # extreme_test
    
    # Generate values with various patterns
    
    # Ultra-fast clock toggling every picosecond
    for t in range(0, 1000, 100):  # Toggle every 100ps for 1ns
        gen.set_value(fast_clk, t, '0' if (t // 100) % 2 == 0 else '1')
        
    # Random data bursts
    for t in range(0, 1000000, 1000):  # Every nanosecond
        gen.set_value(fast_data, t, format(random.randint(0, 2**32-1), '032b'))
    
    # Wide bus patterns
    for t in range(0, 100000, 10000):
        # Alternating patterns
        if t % 20000 == 0:
            pattern = '10' * 512
        else:
            pattern = '01' * 512
        gen.set_value(wide_1k, t, pattern)
        gen.set_value(wide_4k, t, pattern * 4)
    
    # Deep signal changes
    for t in range(0, 100000, 5000):
        gen.set_value(deep_signal, t, format(t % 256, '08b'))
    
    # Many signals with different patterns
    for t in range(0, 10000, 100):
        for i, sig in enumerate(many_sigs[:50]):  # Update half of them frequently
            if random.random() > 0.7:  # 30% chance of change
                width = gen.signals[sig]['width']
                value = format(random.randint(0, 2**width-1), f'0{width}b')
                gen.set_value(sig, t, value)
    
    # Special signals
    for t in range(0, 100000, 25000):
        gen.set_value(special1, t, format(random.randint(0, 65535), '016b'))
        gen.set_value(special2, t, format(random.randint(0, 255), '08b'))
        gen.set_value(special3, t, format(random.randint(0, 15), '04b'))
    
    gen.save()

def generate_protocol_test():
    """Generate a waveform simulating various protocols"""
    gen = WaveformGenerator("protocol_test.vcd", "1ns")
    
    gen.add_scope("protocols")
    
    # I2C-like signals
    gen.add_scope("i2c")
    sda = gen.add_signal("sda", 1)
    scl = gen.add_signal("scl", 1)
    gen.exit_scope()
    
    # SPI-like signals
    gen.add_scope("spi")
    mosi = gen.add_signal("mosi", 1)
    miso = gen.add_signal("miso", 1)
    sclk = gen.add_signal("sclk", 1)
    cs = gen.add_signal("cs", 1)
    gen.exit_scope()
    
    # UART-like signals
    gen.add_scope("uart")
    tx = gen.add_signal("tx", 1)
    rx = gen.add_signal("rx", 1)
    gen.exit_scope()
    
    # Memory interface
    gen.add_scope("memory")
    addr = gen.add_signal("address", 32)
    data = gen.add_signal("data", 64)
    we = gen.add_signal("write_enable", 1)
    re = gen.add_signal("read_enable", 1)
    gen.exit_scope()
    
    # Bus with transactions
    gen.add_scope("axi")
    awaddr = gen.add_signal("awaddr", 32)
    wdata = gen.add_signal("wdata", 64)
    wstrb = gen.add_signal("wstrb", 8)
    awvalid = gen.add_signal("awvalid", 1)
    awready = gen.add_signal("awready", 1)
    gen.exit_scope()
    
    gen.exit_scope()
    
    # Generate realistic protocol patterns
    
    # I2C start condition, data, stop
    i2c_times = [0, 100, 200, 300, 400, 500, 600, 700, 800, 900, 1000]
    for i, t in enumerate(i2c_times):
        gen.set_value(scl, t, '1' if i % 2 == 0 else '0')
        gen.set_value(sda, t, '1' if i % 3 == 0 else '0')
    
    # SPI burst transfer
    gen.set_value(cs, 0, '1')
    gen.set_value(cs, 1000, '0')  # Chip select active
    for t in range(1000, 2000, 50):
        gen.set_value(sclk, t, '1' if ((t-1000) // 50) % 2 == 0 else '0')
        gen.set_value(mosi, t, '1' if random.random() > 0.5 else '0')
        gen.set_value(miso, t, '1' if random.random() > 0.5 else '0')
    gen.set_value(cs, 2000, '1')  # Chip select inactive
    
    # UART transmission (8N1 format)
    uart_byte = '10101100'
    gen.set_value(tx, 0, '1')  # Idle
    gen.set_value(tx, 3000, '0')  # Start bit
    for i, bit in enumerate(uart_byte):
        gen.set_value(tx, 3100 + i*100, bit)
    gen.set_value(tx, 3900, '1')  # Stop bit
    
    # Memory transactions
    for t in range(0, 10000, 500):
        gen.set_value(addr, t, format(0x80000000 + t*4, '032b'))
        gen.set_value(data, t, format(random.randint(0, 2**64-1), '064b'))
        gen.set_value(we, t, '1' if t % 1000 == 0 else '0')
        gen.set_value(re, t, '1' if t % 1000 == 500 else '0')
    
    # AXI write transaction
    gen.set_value(awvalid, 5000, '0')
    gen.set_value(awready, 5000, '1')
    gen.set_value(awaddr, 5100, format(0xDEADBEEF, '032b'))
    gen.set_value(awvalid, 5100, '1')
    gen.set_value(wdata, 5200, format(0x123456789ABCDEF0, '064b'))
    gen.set_value(wstrb, 5200, '11111111')
    gen.set_value(awvalid, 5300, '0')
    
    gen.save()

def generate_performance_test():
    """Generate a massive waveform for performance testing"""
    gen = WaveformGenerator("performance_test.vcd", "1ns")
    
    gen.add_scope("performance")
    
    # Create many signals
    signals = []
    for module_idx in range(10):
        gen.add_scope(f"module_{module_idx}")
        for sig_idx in range(50):
            width = [1, 4, 8, 16, 32][sig_idx % 5]
            sig = gen.add_signal(f"signal_{sig_idx:02d}", width)
            signals.append((sig, width))
        gen.exit_scope()
    
    gen.exit_scope()
    
    # Generate many value changes
    print("Generating performance test with 500 signals and 100,000 time points...")
    for t in range(0, 100000, 10):
        # Change 10% of signals at each time point
        for sig, width in random.sample(signals, len(signals) // 10):
            value = format(random.randint(0, 2**width-1), f'0{width}b')
            gen.set_value(sig, t, value)
        
        if t % 10000 == 0:
            print(f"  Generated up to time {t}...")
    
    gen.save()

def generate_complex_vcd():
    """Generate corrected complex.vcd file with valid binary signals only"""
    gen = WaveformGenerator("complex.vcd", "1s")
    
    gen.add_scope("top")
    
    # CPU module
    gen.add_scope("cpu")
    clk = gen.add_signal("clk", 1)
    reset = gen.add_signal("reset", 1)
    pc = gen.add_signal("pc", 32)
    data_bus = gen.add_signal("data_bus", 64)
    wide_bus = gen.add_signal("wide_bus", 128)
    opcode = gen.add_signal("opcode", 8)
    counter = gen.add_signal("counter", 16, "reg")
    overflow = gen.add_signal("overflow", 1)
    gen.exit_scope()
    
    # Memory module
    gen.add_scope("memory")
    addr = gen.add_signal("addr", 32)
    mem_data = gen.add_signal("mem_data", 8)
    we = gen.add_signal("we", 1)
    oe = gen.add_signal("oe", 1)
    nibble = gen.add_signal("nibble", 4)
    huge_bus = gen.add_signal("huge_bus", 256)
    gen.exit_scope()
    
    # Decoder module
    gen.add_scope("decoder")
    segment = gen.add_signal("segment", 7)
    control = gen.add_signal("control", 12)
    valid = gen.add_signal("valid", 1)
    rgb_data = gen.add_signal("rgb_data", 24)
    extended = gen.add_signal("extended", 48)
    gen.exit_scope()
    
    # Debug module - FIXED: exactly 512 bits for massive_debug
    gen.add_scope("debug")
    state = gen.add_signal("state", 8)
    flags = gen.add_signal("flags", 16)
    error = gen.add_signal("error", 1)
    custom = gen.add_signal("custom", 20)
    massive_debug = gen.add_signal("massive_debug", 512)  # EXACTLY 512 bits
    gen.exit_scope()
    
    gen.exit_scope()  # top
    
    # Generate initial values at time 0
    gen.set_value(clk, 0, '0')
    gen.set_value(reset, 0, '1')
    gen.set_value(pc, 0, '0' * 32)
    gen.set_value(data_bus, 0, '0' * 64)
    gen.set_value(wide_bus, 0, '0' * 128)
    gen.set_value(opcode, 0, '0' * 8)
    gen.set_value(counter, 0, '0' * 16)
    gen.set_value(overflow, 0, '0')
    gen.set_value(addr, 0, '0' * 32)
    gen.set_value(mem_data, 0, '0' * 8)
    gen.set_value(we, 0, '0')
    gen.set_value(oe, 0, '0')
    gen.set_value(nibble, 0, '0' * 4)
    gen.set_value(huge_bus, 0, '0' * 256)
    gen.set_value(segment, 0, '0' * 7)
    gen.set_value(control, 0, '0' * 12)
    gen.set_value(valid, 0, '0')
    gen.set_value(rgb_data, 0, '0' * 24)
    gen.set_value(extended, 0, '0' * 48)
    gen.set_value(state, 0, '00000001')
    gen.set_value(flags, 0, '0' * 16)
    gen.set_value(error, 0, '0')
    gen.set_value(custom, 0, '0' * 20)
    gen.set_value(massive_debug, 0, '0' * 512)  # EXACTLY 512 bits
    
    # Generate changing values with ONLY binary values
    for t in range(1, 50):
        if t % 2 == 1:
            gen.set_value(clk, t, '1')
            gen.set_value(pc, t, format((t-1)*4, f'0{32}b'))
            gen.set_value(counter, t, format(t, f'0{16}b'))
        else:
            gen.set_value(clk, t, '0')
            
        if t == 2:
            gen.set_value(mem_data, t, '11111111')
            gen.set_value(we, t, '1')
            gen.set_value(nibble, t, '1010')
            
        if t == 3:
            gen.set_value(opcode, t, '10101010')
            
        if t == 4:
            gen.set_value(we, t, '0')
            gen.set_value(nibble, t, '0101')
            gen.set_value(addr, t, '1' * 32)
            
        if t == 5:
            gen.set_value(oe, t, '1')
            gen.set_value(nibble, t, '1111')
            gen.set_value(data_bus, t, '1' * 64)
            
        if t == 10:
            gen.set_value(massive_debug, t, '1' * 512)  # All ones pattern
            
        if t == 15:
            gen.set_value(massive_debug, t, '0' * 512)  # Back to zeros
            
        if t == 20:
            gen.set_value(massive_debug, t, '10' * 256)  # Alternating pattern
    
    gen.save()

def generate_stress_test_vcd():
    """Generate corrected stress_test.vcd file with ONLY binary values (no x/z)"""
    gen = WaveformGenerator("stress_test.vcd", "1ns")
    
    gen.add_scope("stress_test")
    
    # Rapid changes module
    gen.add_scope("rapid_changes")
    toggle_fast = gen.add_signal("toggle_fast", 1)
    byte_pattern = gen.add_signal("byte_pattern", 8)
    word_counter = gen.add_signal("word_counter", 16)
    random_data = gen.add_signal("random_data", 32)
    gen.exit_scope()
    
    # Edge cases module - FIXED: NO four-state values
    gen.add_scope("edge_cases")
    x_state = gen.add_signal("x_state", 1)      # FIXED: only '0' and '1'
    z_state = gen.add_signal("z_state", 1)      # FIXED: only '0' and '1'
    unknown_byte = gen.add_signal("unknown_byte", 8)  # FIXED: only binary
    mixed_states = gen.add_signal("mixed_states", 4)  # FIXED: only binary
    gen.exit_scope()
    
    # Long values module
    gen.add_scope("long_values")
    kilobit_bus = gen.add_signal("kilobit_bus", 1024)
    two_kilobit = gen.add_signal("two_kilobit", 2048)
    gen.exit_scope()
    
    # Strings module
    gen.add_scope("strings")
    ascii_text = gen.add_signal("ascii_text", 64)
    status_code = gen.add_signal("status_code", 32)
    gen.exit_scope()
    
    gen.exit_scope()  # stress_test
    
    # Initial values (time 0) - ALL BINARY
    gen.set_value(toggle_fast, 0, '0')
    gen.set_value(byte_pattern, 0, '0' * 8)
    gen.set_value(word_counter, 0, '0' * 16)
    gen.set_value(random_data, 0, '0' * 32)
    gen.set_value(x_state, 0, '0')          # FIXED: binary only
    gen.set_value(z_state, 0, '0')          # FIXED: binary only
    gen.set_value(unknown_byte, 0, '0' * 8) # FIXED: binary only
    gen.set_value(mixed_states, 0, '0' * 4) # FIXED: binary only
    gen.set_value(kilobit_bus, 0, '0' * 1024)
    gen.set_value(two_kilobit, 0, '0' * 2048)
    # ASCII strings must be padded to exact signal width
    # ascii_text: 64-bit signal = 8 ASCII characters
    # "Hello" (5 chars) + 3 padding null chars = 8 total chars = 64 bits
    hello_padded = '0100100001100101011011000110110001101111' + '000000000000000000000000'  # "Hello" + 3 nulls
    gen.set_value(ascii_text, 0, hello_padded)
    
    # status_code: 32-bit signal = 4 ASCII characters  
    # "OKA" (3 chars) + 1 control char (0x19) = 4 total chars = 32 bits (already correct)
    gen.set_value(status_code, 0, '01001111010010110100000100011001')
    
    # Generate rapid changes with ONLY binary values
    for t in range(1000, 10000, 1000):
        gen.set_value(toggle_fast, t, '1' if (t // 1000) % 2 == 1 else '0')
        gen.set_value(byte_pattern, t, format(random.randint(0, 255), '08b'))
        gen.set_value(word_counter, t, format(t // 1000, '016b'))
        gen.set_value(random_data, t, format(random.randint(0, 2**32-1), '032b'))
        
        # FIXED: Only binary transitions for edge case signals
        if t == 5000:
            gen.set_value(x_state, t, '1')         # FIXED: '1' instead of 'x'
            gen.set_value(z_state, t, '1')         # FIXED: '1' instead of 'z'
            gen.set_value(unknown_byte, t, '00001111')  # FIXED: binary pattern
            gen.set_value(mixed_states, t, '1010')      # FIXED: binary pattern
    
    gen.save()

if __name__ == "__main__":
    print("Generating CORRECTED test waveform files (binary values only)...")
    
    # Generate corrected test files to replace broken ones
    generate_complex_vcd()
    generate_stress_test_vcd()
    
    # Also generate additional test files
    generate_extreme_test()
    generate_protocol_test()
    generate_performance_test()
    
    print("\nGeneration complete!")
    print("\nFixed files:")
    print("1. complex.vcd - corrected massive_debug to exactly 512 bits")
    print("2. stress_test.vcd - removed x/z four-state values, binary only")
    print("\nAdditional test files:")
    print("3. extreme_test.vcd - for edge case testing")
    print("4. protocol_test.vcd - for realistic protocol waveforms")
    print("5. performance_test.vcd - for rendering performance testing")
    print("\nTo convert to FST format, install gtkwave and run:")
    print("  vcd2fst <input.vcd> <output.fst>")