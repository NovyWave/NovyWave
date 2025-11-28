# SpinalHDL Counter Example Makefile
# Requires: sbt (Scala Build Tool), Java 11+

.PHONY: all sim verilog vhdl clean help

# Default target: run simulation to generate VCD
all: sim

# Run simulation (generates VCD waveform)
sim:
	@echo "Running SpinalHDL simulation..."
	sbt "runMain counter.CounterSim"
	@echo ""
	@echo "VCD file generated in: simWorkspace/Counter/test/"
	@if [ -f simWorkspace/Counter/test/wave.vcd ]; then \
		cp simWorkspace/Counter/test/wave.vcd counter.vcd; \
		echo "Copied to: counter.vcd"; \
	fi

# Generate Verilog RTL
verilog:
	@echo "Generating Verilog RTL..."
	sbt "runMain counter.CounterVerilog"
	@echo "Verilog files generated in: rtl/"

# Generate VHDL RTL
vhdl:
	@echo "Generating VHDL RTL..."
	sbt "runMain counter.CounterVhdl"
	@echo "VHDL files generated in: rtl/"

# Clean build artifacts
clean:
	rm -rf target project/target simWorkspace rtl
	rm -f counter.vcd *.fst

# Show help
help:
	@echo "SpinalHDL Counter Example"
	@echo ""
	@echo "Targets:"
	@echo "  all      - Run simulation (default)"
	@echo "  sim      - Run simulation, generate VCD waveform"
	@echo "  verilog  - Generate Verilog RTL"
	@echo "  vhdl     - Generate VHDL RTL"
	@echo "  clean    - Remove build artifacts"
	@echo ""
	@echo "Requirements:"
	@echo "  - Java 11 or later"
	@echo "  - sbt (Scala Build Tool)"
	@echo "  - Verilator (for simulation)"
