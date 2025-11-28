# GHDL Makefile for Counter Example
# Generates a GHW waveform file for NovyWave

# Tool configuration
GHDL = ghdl
GHDL_FLAGS = --std=08

# Source files
SOURCES = counter.vhd counter_tb.vhd

# Output files
WORK_DIR = work
GHW_FILE = counter.ghw

.PHONY: all clean simulate

all: $(GHW_FILE)

# Analyze source files
$(WORK_DIR):
	mkdir -p $(WORK_DIR)
	$(GHDL) -a $(GHDL_FLAGS) --workdir=$(WORK_DIR) $(SOURCES)

# Elaborate the testbench
elaborate: $(WORK_DIR)
	$(GHDL) -e $(GHDL_FLAGS) --workdir=$(WORK_DIR) counter_tb

# Run simulation and generate GHW waveform
$(GHW_FILE): elaborate
	$(GHDL) -r $(GHDL_FLAGS) --workdir=$(WORK_DIR) counter_tb --wave=$(GHW_FILE)

# Convenience target for simulation
simulate: $(GHW_FILE)
	@echo "Waveform generated: $(GHW_FILE)"
	@echo "Open with NovyWave to view the simulation results"

# Clean build artifacts
clean:
	rm -rf $(WORK_DIR) $(GHW_FILE) *.o *.cf
	@echo "Cleaned build artifacts"

# Help
help:
	@echo "GHDL Counter Example Makefile"
	@echo ""
	@echo "Targets:"
	@echo "  all       - Build and run simulation (default)"
	@echo "  simulate  - Same as 'all'"
	@echo "  clean     - Remove build artifacts"
	@echo "  help      - Show this help"
	@echo ""
	@echo "Requirements: GHDL (https://ghdl.github.io/ghdl/)"
