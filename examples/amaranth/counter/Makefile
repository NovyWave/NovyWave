# Amaranth Counter Example Makefile
# Requires: Python 3.8+, pip

.PHONY: all sim verilog setup clean help

# Use virtual environment python if available, else system python
PYTHON := $(shell if [ -f .venv/bin/python ]; then echo .venv/bin/python; else echo python3; fi)
PIP := $(shell if [ -f .venv/bin/pip ]; then echo .venv/bin/pip; else echo pip3; fi)

# Default target: run simulation to generate VCD
all: sim

# Run simulation (generates VCD waveform)
sim:
	@echo "Running Amaranth simulation..."
	$(PYTHON) counter.py
	@echo ""
	@echo "VCD file generated: counter.vcd"

# Generate Verilog RTL
verilog:
	@echo "Generating Verilog RTL..."
	$(PYTHON) counter.py verilog

# Install dependencies (creates virtual environment)
setup:
	@echo "Setting up Amaranth virtual environment..."
	python3 -m venv .venv
	.venv/bin/pip install amaranth
	@echo ""
	@echo "Setup complete! Virtual environment created at .venv/"

# Clean generated files
clean:
	rm -f counter.vcd counter.gtkw counter.v
	rm -rf __pycache__

# Show help
help:
	@echo "Amaranth Counter Example"
	@echo ""
	@echo "Targets:"
	@echo "  all      - Run simulation (default)"
	@echo "  sim      - Run simulation, generate VCD waveform"
	@echo "  verilog  - Generate Verilog RTL"
	@echo "  setup    - Install Python dependencies"
	@echo "  clean    - Remove generated files"
	@echo ""
	@echo "Requirements:"
	@echo "  - Python 3.8 or later"
	@echo "  - pip (Python package manager)"
	@echo ""
	@echo "Quick start:"
	@echo "  make setup  # Install dependencies (once)"
	@echo "  make        # Generate waveform"
