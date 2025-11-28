# Icarus Verilog Makefile for Counter Example
# Generates a VCD waveform file for NovyWave

# Tool configuration
IVERILOG = iverilog
VVP = vvp

# Source files
SOURCES = counter.v counter_tb.v

# Output files
SIM_FILE = counter_sim
VCD_FILE = counter.vcd

.PHONY: all clean simulate

all: $(VCD_FILE)

# Compile Verilog sources
$(SIM_FILE): $(SOURCES)
	$(IVERILOG) -o $(SIM_FILE) $(SOURCES)

# Run simulation and generate VCD waveform
$(VCD_FILE): $(SIM_FILE)
	$(VVP) $(SIM_FILE)

# Convenience target for simulation
simulate: $(VCD_FILE)
	@echo "Waveform generated: $(VCD_FILE)"
	@echo "Open with NovyWave to view the simulation results"

# Clean build artifacts
clean:
	rm -f $(SIM_FILE) $(VCD_FILE)
	@echo "Cleaned build artifacts"

# Help
help:
	@echo "Icarus Verilog Counter Example Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  all       - Build and run simulation (default)"
	@echo "  simulate  - Same as 'all'"
	@echo "  clean     - Remove build artifacts"
	@echo "  help      - Show this help"
	@echo ""
	@echo "Requirements: Icarus Verilog (http://iverilog.icarus.com/)"
