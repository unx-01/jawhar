# jawhar kernel documentation

These pages describe the code as it exists in this tree. Paths such as
`arch/init.rs` are relative to the repository root.

## Start here

- [Build and run](build.md) — toolchain, Makefile targets, QEMU, output files.
- [Boot sequence](boot.md) — from `_start` to the interactive TTY.
- [Architecture layer](arch.md) — x86, x86-64 and AArch64 support code.
- [SMP](smp.md) — CPU table, AP bring-up, LAPIC and PSCI startup.
- [GDT](gdt.md) — x86/x86-64 segment descriptors and boot/trampoline tables.
- [POSIX-style interfaces](posix.md) — which Unix conventions are followed and which are missing.
- [Status and limitations](status.md) — what is implemented and what is still a stub.

## Kernel interfaces

- [System calls](syscall.md) — call numbers, dispatch, macros, return values.
- [errno](errno.md) — error codes and the global error variable.
- [Processes and scheduling](process.md) — process table, `fork`, context switch, `wait`, accounting.
- [Memory management](memory.md) — physical page allocator and virtual address spaces.
- [Time and clocks](time.md) — ticks, milliseconds, microseconds, wall clock, `Tms`.

## Drivers and libraries

- [Console](console.md) — VGA text output and cursor handling.
- [TTY](tty.md) — line input, echo and editing keys.
- [Keyboard](keyboard.md) — PS/2 scancodes, modifiers and escape sequences.
- [I/O and serial](io.md) — UART output, port helpers, `print`/`println`.
- [Support library](lib.md) — spinlocks, errno, `printk`, memory helpers.

## Assembly (`doc/asm/<cpu>/<function>.md`)

- [Index](asm/README.md) — conventions and full page list.
- [x86](asm/x86/) — overview plus per-function pages such as [write](asm/x86/write.md).
- [x86-64](asm/x86_64/) — overview plus per-function pages such as [write](asm/x86_64/write.md).
- [AArch64](asm/aarch64/) — overview plus per-function pages such as [write](asm/aarch64/write.md).
- Alias: [asrch64](asm/asrch64/) → AArch64.

Alias: [prossece.md](prossece.md) → [process](process.md).
