# boot(9) — Boot sequence

## NAME

boot - from assembly entry point to the interactive TTY

## SYNOPSIS

`_start` (assembly) → `__init(a0, a1) -> !` → `tty::tty()`

## DESCRIPTION

## Common entry point

Every architecture eventually calls the same Rust entry point:

```rust
#[no_mangle]
pub extern "C" fn __init(a0: usize, a1: usize) -> !
```

The two arguments are the Multiboot magic and Multiboot information pointer on
x86/x86-64. On AArch64 they are the values QEMU placed in `x0`/`x1` and are
currently unused by the memory manager. The panic handler halts the CPU; a
`rust_eh_personality` stub exists to satisfy the compiler.

`__init` runs these stages in order (see `arch/init.rs`):

1. `arch::init()` — per-architecture setup, timer initialization and SMP BSP
   setup. Its duration is measured with `cycles()`.
2. `boot_service(b"pmm", || pmm::pmm_init(a0, a1))`.
3. `boot_service(b"vmm", vmm::vmm_init)`.
4. `boot_service(b"process", process::init)`.
5. `boot_service(b"console", console::console)`.
6. `flush_boot_log()` — writes the collected lines with `printk`.
7. `tty::tty()` — never returns; it loops reading and echoing lines.
8. `arch::halt_forever()` is unreachable while the TTY loop runs, but is the
   terminal branch if the loop ever exits.

`boot_service` times each stage with `millis()`. `log_service` formats a line
like `[      <seconds>.<microseconds>] <name>\n` into a fixed 256-byte
`BOOT_LOG` buffer. The buffer is flushed only after the console service has
run, which is why the lines appear together at the start of the serial log.

## x86-64 (`arch/x86-64/boot.S`)

1. The linker places the image at `1M`; GRUB/QEMU enters `_start` in 32-bit
   protected mode with `EAX = 0x1BADB002` and `EBX` pointing at the Multiboot
   information structure.
2. Magic and info are saved in `.data` (`mb_magic`, `mb_info`).
3. `.bss` is zeroed — the temporary page tables and the 16 KiB stack live there.
4. A temporary `PML4 → PDPT → PD` hierarchy identity-maps the first 1 GiB with
   512 entries of 2 MiB pages.
5. `CR3`, `CR4.PAE`, `EFER.LME` and `CR0.PG` enable long-mode paging.
6. A 64-bit GDT is loaded with `lgdt`, followed by a far jump to `long_mode`.
7. Data segments and `RSP` are set up, the saved Multiboot values are passed in
   `EDI`/`ESI`, and `__init` is called. Interrupts stay masked from the initial
   `cli`; the `sti` in the file only runs if `__init` ever returns.
8. If `__init` returns, the CPU executes `cli; hlt` forever.

`arch/x86-64/gdt.rs` later reloads a Rust-defined GDT with a null, 64-bit code
(selector `0x08`) and data (`0x10`) descriptor.

## x86 (`arch/x86/boot.S`)

1. Same Multiboot header and 1M load address.
2. A small GDT is installed, the data selectors are reloaded and a far jump
   sets `CS`.
3. `.bss` is zeroed and a 16 KiB stack is set up.
4. `__init(magic, info)` is called; on return the CPU halts.
5. The file also contains an `ap_start32` entry and an AP trampoline used by
   the secondary-CPU startup path.

Paging on 32-bit x86 is enabled later by `mm/vmm.rs`: four 4 MiB huge-page
entries cover the low 128 MiB before `CR0.PG` is set.

## AArch64 (`arch/aarch64/boot.S`)

1. `_start` preserves `x0`/`x1`, zeroes `__bss_start..__bss_end` and sets `SP`
   to `stack_top` (16 KiB stack).
2. It branches to `__init`; on return it loops on `wfe`.
3. `ap_entry` sets `SP` from the context passed via PSCI `CPU_ON` and calls
   `ap_entry_rust`.

The linker script places the image at `0x40080000`, inside the QEMU `virt`
machine RAM region (`0x40000000..0x48000000`) used by the physical allocator.
MMU setup is not done in assembly; it happens in `mm/vmm.rs` via `TTBR0_EL1`,
`MAIR_EL1`, `TCR_EL1` and `SCTLR_EL1`.

## What exists after boot

At the point the TTY loop starts, the kernel has a physical page bitmap, page
tables, one process (PID 1), a cleared console and a timer. There is no IDT or
interrupt-driven scheduler yet; see [Status and limitations](status.md).

## EXAMPLES

### `__init` — Rust

```rust
#[no_mangle]
pub extern "C" fn __init(a0: usize, a1: usize) -> ! {
    crate::arch::init();
    // pmm, vmm, process, console ...
    crate::tty::tty();
}
```

### Calling `__init` from C

```c
#include <stdint.h>

/* The assembly entry passes EDI/ESI; this declaration matches that ABI. */
extern void __init(uint32_t magic, uintptr_t info) __attribute__((noreturn));

void c_boot_entry(uint32_t magic, void *info) {
    __init(magic, (uintptr_t)info);   /* never returns */
}
```

## FILES

- `arch/*/boot.S`, `arch/*/linker.ld` — entry code and load addresses.
- `arch/init.rs` — Rust entry point and timed boot stages.

## SEE ALSO

[build](build.md), [arch](arch.md), [memory](memory.md), [process](process.md), [tty](tty.md)
