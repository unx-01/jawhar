ARCH ?= x86-64
VNC ?= :0
ARCHES := x86 x86-64 aarch64
BUILD_DIR := bin
DOC_DIR := doc
DOCS := $(sort $(wildcard $(DOC_DIR)/*.[1-9]))
DOC_MD := $(sort $(wildcard $(DOC_DIR)/*.md) $(wildcard $(DOC_DIR)/*/*.md) $(wildcard $(DOC_DIR)/*/*/*.md))

RUSTC ?= rustc
CLANG ?= clang
LD = ld.lld
OBJCOPY ?= $(or $(shell command -v llvm-objcopy 2>/dev/null),$(shell command -v objcopy 2>/dev/null),objcopy)
SYSROOT := $(shell $(RUSTC) --print sysroot)

rust_target = $(firstword $(foreach t,$(1),$(if $(wildcard $(SYSROOT)/lib/rustlib/$(t)),$(t))))

RUST_TARGET_x86 := $(or $(call rust_target,i686-unknown-linux-gnu i686-linux-android),i686-linux-android)
RUST_TARGET_x86-64 := $(or $(call rust_target,x86_64-unknown-none x86_64-linux-android),x86_64-linux-android)
RUST_TARGET_aarch64 := $(or $(call rust_target,aarch64-unknown-none aarch64-linux-android),aarch64-linux-android)

CLANG_TARGET_x86 := i386-unknown-none-elf
CLANG_TARGET_x86-64 := x86_64-unknown-none-elf
CLANG_TARGET_aarch64 := aarch64-unknown-none-elf

LD_EMU_x86 := elf_i386
LD_EMU_x86-64 := elf_x86_64
LD_EMU_aarch64 := aarch64elf

RUST_FEATURES_x86 := -C target-feature=-ssse3,-sse3,-sse4.1,-sse4.2,-popcnt
RUST_FEATURES_x86-64 := -C target-feature=-ssse3,-sse3,-sse4.1,-sse4.2,-popcnt
RUST_FEATURES_aarch64 :=

QEMU_x86 := $(or $(shell command -v qemu-system-i386 2>/dev/null),$(shell command -v qemu-system-x86_64 2>/dev/null),qemu-system-i386)
QEMU_x86-64 := qemu-system-x86_64
QEMU_aarch64 := qemu-system-aarch64

QEMU_ARGS_x86 := -m 128M
QEMU_ARGS_x86-64 := -m 128M
QEMU_ARGS_aarch64 := -machine virt -cpu cortex-a57 -m 128M

KERNEL_x86 := $(BUILD_DIR)/x86/jawhar.elf
KERNEL_x86-64 := $(BUILD_DIR)/x86-64/jawhar32.elf
KERNEL_aarch64 := $(BUILD_DIR)/aarch64/jawhar.elf

SOURCES := $(shell find . -type f \
	-not -path './bin/*' \
	-not -path './obj/*' \
	-not -path './.git/*' \
	-not -path './doc/*')

.PHONY: all build $(ARCHES) run run-vnc run-% run-vnc-% stop stop-all clean doc docs help

all: $(ARCHES)

build: $(ARCH)

x86: $(BUILD_DIR)/x86/jawhar.elf

x86-64: $(BUILD_DIR)/x86-64/jawhar.elf $(BUILD_DIR)/x86-64/jawhar32.elf

aarch64: $(BUILD_DIR)/aarch64/jawhar.elf

$(BUILD_DIR)/%/boot.o: arch/%/boot.S
	@mkdir -p $(@D)
	$(CLANG) --target=$(CLANG_TARGET_$*) -c $< -o $@

$(BUILD_DIR)/%/init.o: $(SOURCES)
	@mkdir -p $(@D)
	$(RUSTC) --edition 2021 -O --target $(RUST_TARGET_$*) --emit=obj \
		-C panic=abort -C relocation-model=static -C debuginfo=0 \
		$(RUST_FEATURES_$*) --crate-name jawhar arch/init.rs -o $@

$(BUILD_DIR)/%/jawhar.elf: $(BUILD_DIR)/%/boot.o $(BUILD_DIR)/%/init.o arch/%/linker.ld
	@mkdir -p $(@D)
	$(LD) -m $(LD_EMU_$*) -T arch/$*/linker.ld --gc-sections -static -nostdlib \
		-o $@ $(BUILD_DIR)/$*/boot.o $(BUILD_DIR)/$*/init.o

$(BUILD_DIR)/x86-64/jawhar32.elf: $(BUILD_DIR)/x86-64/jawhar.elf
	$(OBJCOPY) -O elf32-i386 $< $@

run: $(KERNEL_$(ARCH))
	$(QEMU_$(ARCH)) $(QEMU_ARGS_$(ARCH)) -kernel $(KERNEL_$(ARCH)) \
		-monitor none -no-reboot -display none -serial stdio

run-vnc: $(KERNEL_$(ARCH))
	@$(MAKE) --no-print-directory stop ARCH=$(ARCH)
	$(QEMU_$(ARCH)) $(QEMU_ARGS_$(ARCH)) -kernel $(KERNEL_$(ARCH)) \
		-monitor none -no-reboot -vnc $(VNC) \
		-serial file:$(BUILD_DIR)/$(ARCH)/serial.log \
		-pidfile $(BUILD_DIR)/$(ARCH)/qemu.pid -daemonize
	@echo "VNC server running on $(VNC)"

run-%:
	@$(MAKE) --no-print-directory $*
	$(QEMU_$*) $(QEMU_ARGS_$*) -kernel $(KERNEL_$*) \
		-monitor none -no-reboot -display none -serial stdio

run-vnc-%:
	@$(MAKE) --no-print-directory $*
	@$(MAKE) --no-print-directory stop ARCH=$*
	$(QEMU_$*) $(QEMU_ARGS_$*) -kernel $(KERNEL_$*) \
		-monitor none -no-reboot -vnc $(VNC) \
		-serial file:$(BUILD_DIR)/$*/serial.log \
		-pidfile $(BUILD_DIR)/$*/qemu.pid -daemonize
	@echo "VNC server running on $(VNC)"

stop:
	@pidfile=$(BUILD_DIR)/$(ARCH)/qemu.pid; \
	if [ -f $$pidfile ]; then \
		pid=$$(cat $$pidfile); \
		if kill -0 $$pid 2>/dev/null; then \
			kill $$pid; \
			for i in 1 2 3 4 5 6 7 8 9 10; do \
				kill -0 $$pid 2>/dev/null || break; \
				sleep 0.1; \
			done; \
			kill -9 $$pid 2>/dev/null || true; \
			echo "stopped QEMU for $(ARCH) (pid $$pid)"; \
		else \
			echo "QEMU for $(ARCH) is not running"; \
		fi; \
		rm -f $$pidfile; \
	else \
		echo "QEMU for $(ARCH) is not running"; \
	fi

stop-all:
	@for arch in $(ARCHES); do $(MAKE) --no-print-directory stop ARCH=$$arch; done

clean:
	@$(MAKE) --no-print-directory stop-all
	@rm -rf $(BUILD_DIR)

doc docs:
	@for f in $(DOCS) $(DOC_MD); do echo "$$f"; done

help:
	@printf '%s\n' \
		'Usage: make [target] [ARCH=x86|x86-64|aarch64] [VNC=:0]' \
		'' \
		'  all          Build every architecture (default)' \
		'  build        Build ARCH (default: x86-64)' \
		'  x86          Build the 32-bit x86 kernel' \
		'  x86-64       Build the 64-bit x86 kernel' \
		'  aarch64      Build the AArch64 kernel' \
		'  run          Build if needed, then run ARCH with the serial console' \
		'  run-vnc      Build if needed, then run ARCH with a VNC display' \
		'  run-x86      Run x86 using the serial console' \
		'  run-x86-64   Run x86-64 using the serial console' \
		'  run-aarch64  Run aarch64 using the serial console' \
		'  run-vnc-*    Run the matching architecture in VNC' \
		'  docs         List kernel documentation (doc/)' \
		'  stop         Stop the QEMU VNC instance for ARCH' \
		'  stop-all     Stop all QEMU VNC instances' \
		'  clean        Stop QEMU and remove all build outputs' \
		'  help         Show this help'
