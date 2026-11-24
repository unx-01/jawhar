# memory(9) — Memory management

## NAME

memory - physical page allocator and virtual address spaces

## SYNOPSIS

`pmm::alloc_page()`; `AddressSpace::new()`; `AddressSpace::map(virt, phys, flags)`

## DESCRIPTION

Two layers: `mm/pmm.rs` (physical pages) and `mm/vmm.rs` (virtual address
spaces). `mm/mem.rs` provides small libc-style helpers.

## Physical memory manager (`pmm`)

Page size is 4096 bytes. A static bitmap holds one bit per page; a set bit means
"used". The bitmap starts as all ones, then usable regions are marked free.

| Architecture | `MAX_PHYS` | Initial free region |
| --- | --- | --- |
| x86 | 128 MiB | `1 MiB .. 128 MiB` |
| x86-64 | 1 GiB | `1 MiB .. 128 MiB` (no Multiboot map) |
| AArch64 | `0x48000000` (1152 MiB) | `0x40000000 .. 0x48000000` |

`pmm_init(magic, info)`:

- On x86/x86-64, if `magic == 0x2BADB002`, flag 6 is set and the Multiboot
  memory map is present, each type-1 (available) region is freed. The map itself
  and the Multiboot info page are marked used. Without a valid map, the 1–128 MiB
  region is freed and the info page is reserved if non-null.
- On AArch64, the fixed QEMU `virt` RAM window is freed.
- `reserve_kernel()` marks `0 .. __bss_end` as used.
- Everything below `LOW_MEMORY = 0x100000` is skipped when freeing regions.

API:

| Function | Behavior |
| --- | --- |
| `alloc_page()` | Linear scan from `NEXT_HINT` for a free bit; returns the physical address or `None`. |
| `alloc_pages(count)` | Finds a contiguous run of `count` free pages; returns its start address or `None`. |
| `free_page(addr)`, `free_pages(addr, count)` | Clear the corresponding bits. |
| `mark_used(addr, size)`, `mark_free(addr, size)` | Explicit reservation helpers. |
| `total_pages()`, `free_page_count()`, `used_pages()` | Bitmap statistics. |

The counters and hint are atomics, but the bitmap read/modify/write sequences
are not protected by a lock.

## Virtual memory manager (`vmm`)

`vmm_init()` calls the architecture-specific `init()`:

- **x86-64**: enables NX in `EFER`, allocates a PML4/PDPT/PD, maps the first
  1 GiB with 2 MiB huge pages and loads `CR3`. `invlpg` flushes single pages;
  reloading `CR3` flushes all. User-space mappings must use PML4 indices 1–255
  (`USER_FIRST = 1`, `USER_LAST = 256`).
- **x86**: allocates a page directory, maps the low 128 MiB with 4 MiB huge
  pages (entries 0–31), enables `CR4.PSE` and paging via `CR0.PG`. Page tables
  for user mappings start at directory index 32. There is no NX support.
- **AArch64**: builds a two-level setup in `TTBR0_EL1` with device and normal
  memory attributes in `MAIR_EL1`, `TCR_EL1` configuration and `SCTLR_EL1`
  bits for MMU/alignment/caches. Large blocks (1 GiB/2 MiB) are split when a
  finer mapping is needed; TLB invalidation uses `tlbi vmalle1`.

`Flags` controls leaf permissions: `writable`, `user`, `executable`. Prebuilt
constants are `KERNEL_RO`, `KERNEL_RX`, `KERNEL_RW`, `KERNEL_RWX` and the
matching `USER_*` values. x86-64 encodes non-executable pages with the NX bit;
AArch64 uses PXN/UXN.

`AddressSpace` is the main API:

| Method | Meaning |
| --- | --- |
| `AddressSpace::kernel()` / `current()` | Wraps the currently loaded root (`CR3`/`TTBR0_EL1`). |
| `AddressSpace::new()` | Allocates a root table and copies the kernel entries from the current one. |
| `map(virt, phys, flags)` | Adds a 4 KiB mapping, allocating intermediate tables as needed; rejects unaligned addresses and user mappings in kernel ranges. |
| `unmap(virt)` | Removes a mapping and, for user spaces, frees empty intermediate tables; returns the old physical address. |
| `translate(virt)` | Walks the tables and returns the physical address, including huge-page offsets. |
| `activate()` | Loads the root into `CR3`/`TTBR0_EL1`. |
| `destroy()` | Frees tables and root; only acts on user spaces created by `new()`. |

Huge pages present in the inherited kernel map are split on demand when a
sub-page mapping or unmap is requested. Empty user-space tables are reclaimed
on `unmap`.

Module-level helpers `map_page`, `unmap_page` and `translate` operate on the
current kernel root directly.

`mm/mem.rs` exports `memcpy`, `memset`, `memmove` and `memcmp` with C symbols.
`arch/init.rs` references `mem::memcpy` so `--gc-sections` keeps the symbol.

## Current scope

`AddressSpace::new()`/`activate()` exist but `process::fork()` does not call
them yet; forked processes currently share the single kernel page table. See
[Status and limitations](status.md).

## EXAMPLES

### `pmm::alloc_pages` + `AddressSpace` — Rust

```rust
if let Some(phys) = crate::pmm::alloc_pages(1) {
    crate::pmm::mark_used(phys, crate::pmm::PAGE_SIZE);

    let mut space = crate::vmm::AddressSpace::new().expect("page tables");
    if space.map(0x4000_0000, phys, crate::vmm::USER_RW) {
        space.activate();
        assert_eq!(space.translate(0x4000_0000), Some(phys));
    }
}
```

### `pmm::free_page` — Rust

```rust
let page = crate::pmm::alloc_page().expect("no free page");
// ... use page ...
crate::pmm::free_page(page);
```

### Activating a page-table root — C

```c
#include <stdint.h>

/* Exported from Rust: AddressSpace::activate() / vmm::flush_addr(). */
extern void vmm_activate(uintptr_t phys_root);
extern void vmm_flush_addr(uintptr_t virt);

vmm_activate(root_phys);              /* writes CR3 / TTBR */
vmm_flush_addr(0x40000000);           /* invalidate one address */
```

### Writing a leaf PTE — C

```c
#include <stdint.h>

volatile uint64_t *pte = (volatile uint64_t *)pte_addr;
*pte = 0x1003;   /* phys 0x1000, flags 0x3 = present | writable */
```

## FILES

- `mm/pmm.rs`, `mm/vmm.rs`, `mm/mem.rs`.
- `arch/*/` — architecture page-table implementations.

## SEE ALSO

[process](process.md), [arch](arch.md), [status](status.md), [gdt](gdt.md)
