# arch(9) — Architecture layer

## NAME

arch - hardware abstraction layer for x86, x86-64 and AArch64

## SYNOPSIS

`arch::init()`; `arch::halt_forever() -> !`

## DESCRIPTION

`arch/` is the hardware abstraction layer. `arch/init.rs` is the crate root and
pulls in every other module with `#[path]` attributes. `arch/mod.rs` selects the
per-architecture implementation with `#[cfg(target_arch = ...)]`.

## Public interface (`arch/mod.rs`, `arch/smp.rs`, `arch/time.rs`)

| Item | Purpose |
| --- | --- |
| `arch::init()` | Calls the architecture init, enables SSE/SSE2 on x86/x86-64, then `time::init()` and `smp::init()`. |
| `arch::halt_forever()` | `cli; hlt` loop on x86/x86-64, `wfe` loop on AArch64. |
| `arch::time` | `ticks()`, `millis()`, `micros()`, `cycles()`, `wall_seconds()` and conversion helpers. See [Time and clocks](time.md). |
| `arch::smp` | CPU bookkeeping: `MAX_CPUS = 8`, `cpu_id()`, `cpu_count()`, `online_cpus()`, `bsp_id()`, `is_bsp()`, logical CPU allocation and info slots. See [SMP](smp.md). |

`arch/smp::init()` clears the CPU table, marks CPU 0 online and calls the
architecture-specific `init_bsp()`. Secondary CPUs are brought up by the
per-architecture SMP module; the generic table is shared.

## x86 (`arch/x86/`)

- `boot.S` — Multiboot entry, temporary GDT, AP trampoline.
- `gdt.rs` — 32-bit GDT setup for protected mode.
- `lapic.rs` — MMIO access to the local APIC at `0xFEE00000` (ID, EOI,
  INIT/STARTUP IPIs, spurious vector).
- `smp.rs`, `ap_startup.rs` — BSP discovery and application-processor startup
  through the low-memory trampoline.
- `linker.ld` — image base `1M`.

## x86-64 (`arch/x86-64/`)

- `boot.S` — Multiboot entry that switches to 64-bit long mode with a temporary
  identity map of the first 1 GiB.
- `gdt.rs` — three descriptors: null, kernel code (`0x08`, long mode), kernel
  data (`0x10`). `init()` executes `lgdt` and reloads the data segments. See
  [GDT](gdt.md).
- `lapic.rs` — same register interface as the 32-bit version: `init(base)`,
  `id()`, `eoi()`, `send_init()`, `send_startup()`, `send_ipi()`.
- `smp.rs`, `ap_startup.rs` — AP bring-up using LAPIC INIT/STARTUP IPIs and a
  trampoline.
- `linker.ld` — image base `1M`.

`arch::init()` calls `x86_64::init()` (GDT) and then `enable_sse64()`, which sets
CR4 bits 9 and 10 (OSFXSR, OSXMMEXCPT).

## AArch64 (`arch/aarch64/`)

- `boot.S` — entry and `ap_entry` for PSCI-started CPUs.
- `mod.rs` — reads `CurrentEL`, enables FP/SIMD access through `CPACR_EL1`
  (bits 20–21) and issues `isb`. There is no GDT on AArch64.
- `smp.rs` — reads `MPIDR_EL1`, records the BSP ID, and starts secondary CPUs
  with a PSCI `CPU_ON` call (`hvc #0`, function ID `0xC4000003`).
  `ap_entry_rust()` assigns a logical CPU ID, records it and waits in `wfe`.
- `linker.ld` — image base `0x40080000`.

## Notes

- The architecture modules are not public userspace APIs; there is no syscall
  that exposes CPU or timer state directly beyond [times/clock](syscall.md).
- Secondary CPU entry points park in `wfe`/`hlt`; work is not distributed to
  them by the process scheduler.

## EXAMPLES

### `arch::init` and `halt_forever` — Rust

```rust
crate::arch::init();              // GDT/MMU, timer, SMP, SSE
// ... kernel work ...
crate::arch::halt_forever();      // never returns
```

### `halt_forever` — C

```c
/* Exported from Rust with #[no_mangle] extern "C". */
extern void arch_init(void);
extern void arch_halt_forever(void) __attribute__((noreturn));

void c_entry(void) {
    arch_init();
    arch_halt_forever();          /* never returns */
}
```

### Enabling SSE/SSE2 — C

```c
/* arch::init() owns the privileged CR4 write; expose it as a C function. */
extern void arch_enable_sse(void);   /* sets CR4 OSFXSR | OSXMMEXCPT */

arch_enable_sse();
```

## FILES

- `arch/mod.rs` — architecture selection and `halt_forever`.
- `arch/smp.rs`, `arch/time.rs` — generic CPU and timer interfaces.
- `arch/x86/`, `arch/x86-64/`, `arch/aarch64/` — per-architecture code.

## SEE ALSO

[boot](boot.md), [smp](smp.md), [gdt](gdt.md), [memory](memory.md), [time](time.md)
