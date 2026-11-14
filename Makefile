PYTHON ?= python3
ARCH ?= x86-64
VNC ?= :0
ARCHES := x86 x86-64 aarch64

.PHONY: all build $(ARCHES) run run-vnc stop clean help

all: $(ARCHES)

build:
	$(PYTHON) build.py build $(ARCH)

$(ARCHES):
	$(PYTHON) build.py build $@

run:
	$(PYTHON) build.py run $(ARCH)

run-vnc:
	$(PYTHON) build.py run --vnc $(VNC) $(ARCH)

stop:
	$(PYTHON) build.py stop $(ARCH)

clean:
	$(PYTHON) build.py clean

help:
	@printf '%s\n' \
		'Usage: make [target] [ARCH=x86|x86-64|aarch64] [VNC=:0]' \
		'' \
		'  all       Build every architecture (default)' \
		'  build     Build ARCH (default: x86-64)' \
		'  x86       Build the 32-bit x86 kernel' \
		'  x86-64    Build the 64-bit x86 kernel' \
		'  aarch64   Build the AArch64 kernel' \
		'  run       Build if needed, then run ARCH in QEMU' \
		'  run-vnc   Run ARCH in QEMU with a VNC display' \
		'  stop      Stop the QEMU VNC instance for ARCH' \
		'  clean     Remove build outputs' \
		'  help      Show this help'
