# gdt(9) — GDT

## NAME

gdt - x86 and x86-64 global descriptor tables

## SYNOPSIS

`Desc::new(base, limit, access, flags)`; `gdt::init()` → `lgdt`

## DESCRIPTION

Implementation: `arch/x86/gdt.rs` and `arch/x86-64/gdt.rs`, with additional
boot/trampoline GDTs in `arch/x86/boot.S` and `arch/x86-64/boot.S`.
AArch64 has no GDT; its translation tables and `TTBR_EL1` play a similar role.

## Descriptors

Each GDT entry is an 8-byte packed structure:

| Field | Size | Meaning |
| --- | --- | --- |
| `limit_lo` | 16 bits | Low 16 bits of the limit |
| `base_lo` | 16 bits | Low 16 bits of the base |
| `base_mid` | 8 bits | Middle 8 bits of the base |
| `access` | 8 bits | Present, privilege, type bits |
| `flags_hi` | 8 bits | High 4 limit bits in the low nibble, flags in the high nibble |
| `base_hi` | 8 bits | High 8 bits of the base |

`Desc::new(base, limit, access, flags)` builds an entry. All bases are zero and
the limit is `0xFFFFF`, so with 4 KiB granularity the segments cover 4 GiB
(flat segmentation).

| Selector | Kind | Access | Flags (x86) | Flags (x86-64) |
| --- | --- | --- | --- | --- |
| `0x00` | Null | `0x00` | `0x0` | `0x0` |
| `0x08` | Kernel code, ring 0 | `0x9A` (present, code, readable) | `0xC` (G=1, D=1, 32-bit) | `0xA` (G=1, L=1, long mode) |
| `0x10` | Kernel data, ring 0 | `0x92` (present, data, writable) | `0xC` | `0xC` (G=1, D=1) |

## Initialization

`gdt::init()` builds a `Ptr { limit: size_of::<[Desc; 3]>() - 1, base }` and
executes `lgdt`. The pointer base is `u32` on x86 and `u64` on x86-64. It then
loads `0x10` into `DS`, `ES`, `FS`, `GS` and `SS` with inline assembly. `CS` is
reloaded by the far jump in `boot.S`.

- On x86, `boot.S` installs an equivalent static `gdt` before entering Rust.
- On x86-64, `boot.S` installs `gdt64` before the far jump into long mode;
  `gdt::init()` replaces it with the Rust-defined table during `arch::init()`.

## Trampoline GDTs

For AP startup, `boot.S` contains a relocatable GDT blob copied to physical
`0x8000` and patched by `ap_startup.rs`:

- x86: 3 entries — null, 32-bit code (`0x08`), 32-bit data (`0x10`).
- x86-64: 5 entries — null, 32-bit code (`0x08`), 32-bit data (`0x10`),
  64-bit code (`0x18`), 64-bit data (`0x20`). The 32-bit-to-64-bit far jump
  uses selector `0x18`, and long-mode data segments use `0x20`.

The GDT descriptor inside the trampoline has its base patched to the physical
address of the copied GDT before the LAPIC STARTUP IPI is sent.

## Scope

The table is minimal and flat: there are no ring-3/user descriptors, no TSS,
no LDT, and no per-CPU GDTs. Segmentation is effectively disabled apart from
the code/data distinction and long-mode selector setup. See
[SMP](smp.md) for AP startup and [Memory management](memory.md) for the
paging layer that provides actual protection.

## EXAMPLES

### `gdt::init` — Rust

```rust
// Called once from arch::init() on x86-64.
#[cfg(target_arch = "x86_64")]
crate::arch::x86_64::gdt::init();
```

### Loading the same table — C

The `lgdt` and segment reloads stay in the Rust arch layer; export them:

```rust
#[no_mangle]
pub extern "C" fn gdt_load() {
    crate::arch::x86_64::gdt::init();
}
```

```c
extern void gdt_load(void);   /* lgdt + data-segment reload + CS far jump */

gdt_load();                   /* called once from the C boot path */
```

## FILES

- `arch/x86/gdt.rs`, `arch/x86-64/gdt.rs` — Rust GDT tables.
- `arch/x86/boot.S`, `arch/x86-64/boot.S` — boot and trampoline GDTs.

## SEE ALSO

[smp](smp.md), [arch](arch.md), [memory](memory.md)
