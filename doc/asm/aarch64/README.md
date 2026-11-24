# aarch64(9) — AArch64 assembly

## NAME

aarch64 - `virt` boot entry, PSCI secondary entry and AAPCS64 process-switch assembly

## SYNOPSIS

`arch/aarch64/boot.S`, `arch/aarch64/linker.ld`, `arch/aarch64/mod.rs`;
`_start`, `ap_entry`, `stack_top`;
`fork_save`, `context_switch` in `process/init.rs`

## DESCRIPTION

The AArch64 port has no Multiboot header and no `.code16`/`.code64` switch.
`boot.S` is assembled with `clang --target=aarch64-unknown-none-elf -c`,
linked with `ld.lld -m aarch64elf -T arch/aarch64/linker.ld`, and runs under
`qemu-system-aarch64 -machine virt -cpu cortex-a57`. Assembly syntax is
`mnemonic destination, source`; there are no `%` or `$` prefixes.

### Linker layout

`linker.ld` places the image at `0x40080000`, the QEMU `virt` flash/kernel
load address. `.text` is 4-byte aligned; `.rodata`, `.data` and `.bss` are
page aligned. `__bss_start` and `__bss_end` bracket the zeroed region, which
includes the 16 KiB stack `stack_bottom..stack_top`.

### Boot entry `_start`

| Step | Instruction | Result |
| --- | --- | --- |
| 1 | `mov x19, x0`; `mov x20, x1` | Save firmware boot arguments in callee-saved registers. |
| 2 | `adrp x2, __bss_start`; `add x2, x2, :lo12:__bss_start` | Page-absolute address of `.bss` start. |
| 3 | Same for `__bss_end` into `x3` | End address. |
| 4 | `cmp x2, x3`; `b.hs 2f`; `str xzr, [x2], #8`; `b 1b` | Zero `.bss` eight bytes at a time. |
| 5 | `adrp x0, stack_top`; `add x0, x0, :lo12:stack_top`; `mov sp, x0` | Set the stack pointer. |
| 6 | `mov x0, x19`; `mov x1, x20`; `bl __init` | Call Rust with the original boot arguments. |
| 7 | `3: wfe; b 3b` | Park if `__init` returns. |

`adrp` supplies the high bits of a page-aligned address; `:lo12:` adds the
low 12 bits. This avoids absolute relocations in the linked kernel.

### Secondary CPU entry `ap_entry`

`arch/aarch64/smp.rs` starts APs with PSCI `CPU_ON`, passing the target MPIDR,
the address of `ap_entry` and a stack top. `ap_entry` does:

```asm
    cbz x0, 1f          // no context: park immediately
    mov sp, x0          // use the stack passed by CPU_ON
    bl ap_entry_rust
1:  wfe
    b 1b
```

`ap_entry_rust()` assigns a logical CPU ID, records the MPIDR and waits in
`wfe`.

### Architecture setup in Rust inline assembly

`arch/aarch64/mod.rs::init()` uses `asm!` rather than a `.S` file:

- `mrs <reg>, CurrentEL` — read the current exception level.
- `mrs <reg>, CPACR_EL1`, set bits `[21:20]` to `0b11`, `msr CPACR_EL1, <reg>`
  — enable FP/NEON and `isb` to synchronize.
- MMU setup is deferred; see [memory management](../../memory.md).

`arch/aarch64/smp.rs` reads `mpidr_el1` and masks it with `0x00FFFFFF`.
`psci_cpu_on()` issues `hvc #0` with `x0 = 0xC4000003`, `x1 = target CPU`,
`x2 = entry point`, `x3 = context`.

### `global_asm!` process stubs

For `target_arch = "aarch64"`, `process/init.rs` exports `fork_save` and
`context_switch`. Both reserve 96 bytes with `sub sp, sp, #96` and store
`x19`–`x30` in six `stp` pairs. `context_switch` writes `SP` into `[X0]`,
loads `SP` from `[X1]`, restores the pairs and `ret`s (using `x30`).
`fork_save` additionally shuffles `x1`–`x5` into the argument positions of
`fork_copy`.

### Calling convention (AAPCS64)

| Item | Rule |
| --- | --- |
| Arguments | `x0`–`x7`; further arguments on the stack |
| Return value | `x0` (or `x0`/`x1` for wide values) |
| Callee-saved | `x19`–`x28`, `x29` (frame pointer) |
| Link register | `x30` (`lr`) |
| Stack | 16-byte aligned; `sp` cannot be used as an ordinary operand |
| Call | `bl` for a known symbol, `blr` for an indirect target |

## EXAMPLES

### Calling `__init` — AArch64 assembly (from `boot.S`)

```asm
    adrp x0, stack_top
    add  x0, x0, :lo12:stack_top
    mov  sp, x0
    mov  x0, x19              // a0 = saved boot argument 0
    mov  x1, x20              // a1 = saved boot argument 1
    bl   __init               // never returns in practice
```

### Calling `write` through a C wrapper — AArch64 assembly

```rust
#[no_mangle]
pub extern "C" fn jawhar_syscall(
    n: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize,
) -> isize {
    crate::__syscall!(n, a1, a2, a3, a4, a5)
}
```

```c
#include <stdint.h>
extern intptr_t jawhar_syscall(uint32_t, uintptr_t, uintptr_t,
                               uintptr_t, uintptr_t, uintptr_t);
```

```asm
    .section .rodata
msg:
    .ascii  "hello from jawhar\n"
msg_len = . - msg

    .text
    adrp x2, msg
    add  x2, x2, :lo12:msg    // a2 = buffer
    mov  x3, #msg_len         // a3 = length
    mov  x1, #1               // a1 = stdout
    mov  x0, #1               // n  = SYS_WRITE
    mov  x4, xzr              // a4
    mov  x5, xzr              // a5
    bl   jawhar_syscall
    // x0 = bytes written, or -1
```

### Calling `context_switch` — Rust and C

```rust
// Inside process/init.rs, where the private `extern "C"` declaration is in scope:
context_switch(core::ptr::addr_of_mut!(old_ctx), core::ptr::addr_of!(new_ctx));
```

```c
extern void context_switch(void *old, void *new);
context_switch(&old_ctx, &new_ctx);
```

```asm
    adrp x0, old_ctx
    add  x0, x0, :lo12:old_ctx
    adrp x1, new_ctx
    add  x1, x1, :lo12:new_ctx
    bl   context_switch       // returns on the new stack
```

### Enabling FP/NEON — AArch64 assembly

```asm
    mrs x0, CPACR_EL1
    orr x0, x0, #(3 << 20)    // FPEN = 0b11
    msr CPACR_EL1, x0
    isb
```

### Starting a secondary CPU — PSCI `CPU_ON`

```asm
    movz x0, #0x0003          // low halfword of 0xC4000003
    movk x0, #0xC400, lsl #32 // high halfword; x0 = 0xC4000003
    mov x1, x19               // target MPIDR affinity
    adrp x2, ap_entry
    add  x2, x2, :lo12:ap_entry
    mov x3, x20               // stack top / context
    hvc #0                    // x0 = 0 on success
```

## FILES

- `arch/aarch64/boot.S`, `arch/aarch64/linker.ld`.
- `arch/aarch64/mod.rs`, `arch/aarch64/smp.rs`.
- `process/init.rs` — AArch64 `global_asm!` block.

## SEE ALSO

[x86](../x86/), [x86-64](../x86_64/), [boot](../../boot.md),
[smp](../../smp.md), [memory](../../memory.md), [syscall](../../syscall.md)
