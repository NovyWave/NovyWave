#!/usr/bin/env python3
"""
Amaranth HDL Counter Example

An 8-bit counter with enable and overflow detection,
demonstrating waveform generation for NovyWave.
"""

from amaranth import *
from amaranth.sim import Simulator, Tick


class Counter(Elaboratable):
    """8-bit counter with enable and overflow detection."""

    def __init__(self):
        # Ports
        self.enable = Signal(name="enable")
        self.count = Signal(8, name="count")
        self.overflow = Signal(name="overflow")

    def elaborate(self, platform):
        m = Module()

        # Counter logic
        with m.If(self.enable):
            m.d.sync += self.count.eq(self.count + 1)

        # Overflow detection (count about to wrap from 255 to 0)
        with m.If(self.enable & (self.count == 255)):
            m.d.sync += self.overflow.eq(1)
        with m.Else():
            m.d.sync += self.overflow.eq(0)

        return m


def simulate():
    """Run simulation and generate VCD waveform file."""
    dut = Counter()

    def testbench():
        # Initialize
        yield dut.enable.eq(0)
        for _ in range(5):
            yield Tick()

        # Test 1: Enable counting
        print("Test 1: Enable counting")
        yield dut.enable.eq(1)
        for _ in range(20):
            yield Tick()

        # Test 2: Disable counting
        print("Test 2: Disable counting")
        yield dut.enable.eq(0)
        for _ in range(5):
            yield Tick()

        # Test 3: Resume counting
        print("Test 3: Resume counting")
        yield dut.enable.eq(1)
        for _ in range(10):
            yield Tick()

        # Test 4: Count to overflow
        print("Test 4: Counting to overflow")
        cycles = 0
        while cycles < 300:
            yield Tick()
            cycles += 1
            overflow = yield dut.overflow
            if overflow:
                print(f"  Overflow detected at cycle {cycles}")
                break

        # Continue a bit after overflow
        for _ in range(10):
            yield Tick()

        print("Simulation complete!")

    # Create simulator
    sim = Simulator(dut)
    sim.add_clock(1e-8)  # 100 MHz clock (10ns period)
    sim.add_testbench(testbench)

    # Run simulation with VCD output
    with sim.write_vcd("counter.vcd", gtkw_file="counter.gtkw"):
        sim.run()

    print(f"\nVCD file generated: counter.vcd")
    print("Open in NovyWave: novywave counter.vcd")


def generate_verilog():
    """Generate Verilog RTL output."""
    from amaranth.back import verilog

    dut = Counter()
    output = verilog.convert(dut, ports=[dut.enable, dut.count, dut.overflow])

    with open("counter.v", "w") as f:
        f.write(output)

    print("Verilog file generated: counter.v")


if __name__ == "__main__":
    import sys

    if len(sys.argv) > 1 and sys.argv[1] == "verilog":
        generate_verilog()
    else:
        simulate()
