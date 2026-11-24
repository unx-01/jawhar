# smp(9) — SMP

## NAME

smp - CPU bookkeeping and secondary processor startup

## SYNOPSIS

`smp::init()`; `start_ap(apic_id)`; PSCI `CPU_ON`

## DESCRIPTION

Implementation: `arch/smp.rs` (generic CPU table) plus per-architecture modules
`arch/x86/smp.rs`, `arch/x86/ap_startup.rs`, `arch/x86-64/smp.rs`,
`arch/x86-64/ap_startup.rs`, `arch/aarch64/smp.rs` and the trampolines in
`arch/*/boot.S`.

## Generic CPU table (`arch/smp.rs`)

| Item | Value / meaning |
| --- | --- |
| `MAX_CPUS` | 8 |
| `CpuInfo` | `online`, `apic_id`, `hart_id` (MPIDR on AArch64) |
| `CPU_COUNT`, `ONLINE_CPUS` | Atomic counters, initialized to 1 |
| `BSP_ID` | Atomic bootstrap-processor ID, currently always 0 |
| `NEXT_LOGICAL_CPU` | Atomic allocator starting at 1 |
| `CPUS` | Static array of 8 `CpuInfo` slots |

`smp::init()` clears the table, marks slot 0 online, resets the counters and
calls the architecture's `init_bsp()`. `arch::init()` calls it after timer
initialization.

API:

- `cpu_id()` — delegates to the architecture (APIC ID on x86, MPIDR on AArch64).
- `cpu_count()`, `online_cpus()`, `bsp_id()`, `is_bsp()`.
- `alloc_logical_cpu()` — compare-and-swap allocation of IDs 1..7.
- `mark_online(id)`, `set_cpu_info(id, apic_id, hart_id)`, `cpu_info(id)`.

## BSP identification

| Architecture | Source | Notes |
| --- | --- | --- |
| x86, x86-64 | `CPUID` leaf 1, `EBX[31:24]` (initial APIC ID) | Compared with `BSP_APIC_ID`; `cpu_id()` returns 0 for both BSP and APs at present. |
| AArch64 | `MPIDR_EL1` masked with `0x00FFFFFF` | BSP MPIDR is saved in `BSP_MPIDR`; APs fall back to `(mpidr & 0xFF)` as a logical ID. |

## AP bring-up on x86 / x86-64

Both architectures use the same pattern in `ap_startup.rs`:

1. Copy the relocatable trampoline from `.rodata` (`ap_trampoline_start` to
   `ap_trampoline_end`) to physical address `0x8000`. The blob must be at most
   4096 bytes.
2. Patch the copied fields: GDT descriptor base → copied trampoline GDT,
   `ap_tr_cr3` → kernel page-table root, `ap_tr_stack` → top of 4 pages from
   `pmm::alloc_pages`, and `ap_tr_entry64`/`ap_tr_entry32` → `ap_start64`/
   `ap_start32`.
3. Initialize the LAPIC at `0xFEE00000`, send an INIT IPI, wait, send two STARTUP
   (SIPI) IPIs with vector `0x8` (`0x8000 >> 12`), with spin delays between.
4. Return `smp::ap_started()`, which is set by the AP's Rust entry.

Trampoline execution:

- 16-bit real-mode code disables interrupts, clears segments, loads its own
  GDT, sets `CR0.PE` and far-jumps to 32-bit code.
- 32-bit code reloads data segments, then:
  - **x86**: optionally loads `CR3` and enables `CR0.PG`, then jumps to
    `ap_tr_entry32`.
  - **x86-64**: always loads `CR3`, enables PAE, EFER.LME and `CR0.PG`, far-jumps
    to selector `0x18` (64-bit code), loads 64-bit data selectors (`0x20`),
    then jumps to `ap_tr_entry64`.
- `ap_start32`/`ap_start64` set up a stack and call `ap_entry_rust()`.

`ap_entry_rust()` allocates a logical CPU ID, records the APIC/MPIDR, marks the
CPU online, sets `AP_STARTED` and then parks in a `hlt` loop.

### LAPIC registers used (`lapic.rs`)

| Offset | Register |
| --- | --- |
| `0x020` | APIC ID |
| `0x0B0` | EOI |
| `0x0F0` | Spurious interrupt vector (enable bit 8, vector `0xFF`) |
| `0x300` / `0x310` | ICR low / high |

INIT is sent as a level-triggered assertion followed by a deassert; SIPI is
edge-triggered. `send_ipi()` sends a generic vector.

## AArch64 secondary CPUs

`arch/aarch64/smp.rs` starts CPUs with PSCI `CPU_ON`:

```
fid   = 0xC4000003
x1    = target MPIDR affinity
x2    = ap_entry (assembly)
x3    = stack top
hvc #0
```

`ap_entry` loads the stack from `x0` and calls `ap_entry_rust()`, which assigns
a logical ID, records CPU info, marks it online and waits in `wfe`.

## Limitations

- No ACPI/MADT or MP table parsing: the kernel does not discover how many APs
  exist, and `smp::init()` does not call `start_ap()` automatically.
- `CPU_COUNT` is not updated by AP startup; only `ONLINE_CPUS` is incremented.
- There is no per-CPU storage (e.g. GS base on x86-64), so `cpu_id()` cannot yet
  distinguish APs before they reach `ap_entry_rust()`.
- APs are parked; the [process scheduler](process.md) does not dispatch work to
  them.
- On x86-64, `ap_start64` reads the `.data` symbol `ap_stack_top`, while
  `start_ap()` patches `ap_tr_stack` in the trampoline; these are separate
  slots.
- The process table, VGA state and PMM bitmap lack locks, so enabling SMP for
  real work would need synchronization.

## EXAMPLES

### `alloc_logical_cpu`, `mark_online`, `cpu_id` — Rust

```rust
let id = crate::smp::alloc_logical_cpu().expect("no free CPU slot");
crate::smp::set_cpu_info(id, 1, 1);
crate::smp::mark_online(id);

let current = crate::smp::cpu_id();
let bsp = crate::smp::is_bsp();
```

### `start_ap` — Rust

```rust
// apic_id comes from CPUID/APIC discovery.
let started = unsafe { crate::arch::x86_64::ap_startup::start_ap(apic_id) };
if !started {
    crate::io::println("AP did not come online");
}
```

### Reading the APIC ID — C

```c
#include <stdint.h>

/* Exported from Rust: CPUID leaf 1, EBX[31:24]. */
extern uint32_t smp_read_apic_id(void);

uint32_t apic_id = smp_read_apic_id();
```

### Enabling the LAPIC — C

```c
#include <stdint.h>

volatile uint32_t *svr = (volatile uint32_t *)0xFEE000F0;
*svr = 0x1FF;   /* enable bit 8 | spurious vector 0xFF */
```

## FILES

- `arch/smp.rs` — generic CPU table.
- `arch/x86*/smp.rs`, `arch/x86*/ap_startup.rs`, `arch/*/boot.S` — AP startup.
- `arch/aarch64/smp.rs` — PSCI startup.

## SEE ALSO

[arch](arch.md), [gdt](gdt.md), [process](process.md), [memory](memory.md)
