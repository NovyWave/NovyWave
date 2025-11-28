# Spade Counter Example Makefile
# Requires: spade (cargo install spade-lang), Icarus Verilog

SPADE = spade
IVERILOG = iverilog
VVP = vvp

SPADE_SRC = counter.spade
VERILOG_OUT = counter_gen.v
TESTBENCH = counter_tb.v
SIM_OUT = counter_sim
VCD_FILE = counter.vcd

.PHONY: all sim verilog clean help

# Default target: compile and simulate
all: sim

# Generate Verilog from Spade
verilog: $(VERILOG_OUT)

$(VERILOG_OUT): $(SPADE_SRC)
	@echo "Compiling Spade to Verilog..."
	$(SPADE) $(SPADE_SRC) -o $(VERILOG_OUT)
	@echo "Generated: $(VERILOG_OUT)"

# Run simulation (generates VCD)
sim: $(VCD_FILE)
	@echo ""
	@echo "VCD file generated: $(VCD_FILE)"

$(VCD_FILE): $(VERILOG_OUT) $(TESTBENCH)
	@echo "Compiling with Icarus Verilog (SystemVerilog mode)..."
	$(IVERILOG) -g2012 -o $(SIM_OUT) $(VERILOG_OUT) $(TESTBENCH)
	@echo "Running simulation..."
	$(VVP) $(SIM_OUT)

# Clean generated files
clean:
	rm -f $(VERILOG_OUT) $(SIM_OUT) $(VCD_FILE)
	@echo "Cleaned build artifacts"

# Show help
help:
	@echo "Spade Counter Example"
	@echo ""
	@echo "Targets:"
	@echo "  all      - Compile and simulate (default)"
	@echo "  verilog  - Generate Verilog from Spade"
	@echo "  sim      - Run simulation, generate VCD"
	@echo "  clean    - Remove generated files"
	@echo ""
	@echo "Requirements:"
	@echo "  - Spade compiler (cargo install spade-lang)"
	@echo "  - Icarus Verilog (iverilog)"
	@echo ""
	@echo "Quick start:"
	@echo "  make        # Compile and simulate"
	@echo "  novywave counter.vcd"
