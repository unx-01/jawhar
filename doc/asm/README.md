# Assembly documentation

Assembly pages are organised as `doc/asm/<cpu>/<function>.md`. Each page shows
how one kernel function is reached from that architecture's assembly, with
Rust, C and assembly examples. Paths are relative to the repository root.

## CPUs

- [x86](x86/) — 32-bit Multiboot entry, GDT, AP trampoline, cdecl ABI.
- [x86_64](x86_64/) — long-mode boot, identity page tables, System V ABI.
- [AArch64](aarch64/) — `virt` boot, PSCI secondary entry, AAPCS64 ABI.

Alias: [asrch64](asrch64/) → [aarch64](aarch64/).

## Functions

| Function | Number | x86 | x86-64 | AArch64 |
| --- | --- | --- | --- | --- |
| `write` | 1 | [x86](x86/write.md) | [x86-64](x86_64/write.md) | [AArch64](aarch64/write.md) |
| `read` | 2 | [x86](x86/read.md) | [x86-64](x86_64/read.md) | [AArch64](aarch64/read.md) |
| `open` | 3 | [x86](x86/open.md) | [x86-64](x86_64/open.md) | [AArch64](aarch64/open.md) |
| `close` | 4 | [x86](x86/close.md) | [x86-64](x86_64/close.md) | [AArch64](aarch64/close.md) |
| `fork` | 6 | [x86](x86/fork.md) | [x86-64](x86_64/fork.md) | [AArch64](aarch64/fork.md) |
| `exit` | 7 | [x86](x86/exit.md) | [x86-64](x86_64/exit.md) | [AArch64](aarch64/exit.md) |
| `wait4` | 9 | [x86](x86/wait4.md) | [x86-64](x86_64/wait4.md) | [AArch64](aarch64/wait4.md) |
| `times` | 10 | [x86](x86/times.md) | [x86-64](x86_64/times.md) | [AArch64](aarch64/times.md) |
| `clock` | 11 | [x86](x86/clock.md) | [x86-64](x86_64/clock.md) | [AArch64](aarch64/clock.md) |

The per-CPU `README.md` files describe boot code, linker layout, trampolines,
registers and the `global_asm!` process stubs:

- [x86 overview](x86/README.md)
- [x86-64 overview](x86_64/README.md)
- [AArch64 overview](aarch64/README.md)

## Conventions

| CPU | Assembler | Syntax | Built with |
| --- | --- | --- | --- |
| x86 | GAS/Clang | AT&T (`source, destination`, `%reg`, `$imm`) | `clang --target=i386-unknown-none-elf -c` |
| x86-64 | GAS/Clang | AT&T, `.code32` then `.code64` | `clang --target=x86_64-unknown-none-elf -c` |
| AArch64 | GAS/Clang | ARM64 (`mnemonic dst, src`) | `clang --target=aarch64-unknown-none-elf -c` |

`boot.o` is linked with `ld.lld -m <emu> -T arch/<arch>/linker.ld`, and the
Rust half is linked as `init.o`; see [build](../build.md).

## Shared assembly symbols

| Symbol | Defined in | Purpose |
| --- | --- | --- |
| `_start` | `arch/*/boot.S` | Entry point named by `ENTRY(_start)`. |
| `stack_bottom`, `stack_top` | `arch/*/boot.S` | 16 KiB boot stack in `.bss`. |
| `__bss_start`, `__bss_end` | `arch/*/linker.ld` | Bounds used by the zeroing loop. |
| `__init` | Rust (`arch/init.rs`) | Kernel entry; reached with `call`/`bl`. |
| `ap_entry_rust` | Rust (`arch/smp.rs`) | Secondary-CPU entry. |
| `fork_save`, `context_switch` | `process/init.rs` (`global_asm!`) | Process context save/restore. |

## SEE ALSO

[Boot sequence](../boot.md), [Architecture layer](../arch.md),
[SMP](../smp.md), [GDT](../gdt.md), [System calls](../syscall.md)
