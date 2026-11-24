# x86_64(9) — x86-64 assembly

## NAME

x86_64 - long-mode boot, identity page tables, AP trampoline and SysV context-switch assembly

## SYNOPSIS

`arch/x86-64/boot.S`, `arch/x86-64/linker.ld`;
`_start`, `long_mode`, `ap_start64`, `ap_trampoline_start`, `gdt64`;
`fork_save`, `context_switch` in `process/init.rs`

## DESCRIPTION

The x86-64 port uses AT&T/GAS syntax and starts in `.code32` before switching
to `.code64`. `boot.S` is assembled with
`clang --target=x86_64-unknown-none-elf -c` and linked with
`ld.lld -m elf_x86_64 -T arch/x86-64/linker.ld`. The image is based at `1M`;
QEMU is given `bin/x86-64/jawhar32.elf`, an `elf32-i386` copy produced by
`objcopy` from the linked ELF.

### Boot entry: 32-bit to long mode

`_start` runs in 32-bit protected mode with `EAX` = Multiboot magic and
`EBX` = info pointer.

| Step | Operation | Details |
| --- | --- | --- |
| 1 | `cli`; save `EAX`/`EBX` to `mb_magic`/`mb_info` | Same Multiboot header as the 32-bit port. |
| 2 | `rep stosb` over `__bss_start..__bss_end` | Zeroes `pml4`, `pdpt`, `pd` and the stack. |
| 3 | `PML4[0] = pdpt | 3`, `PDPT[0] = pd | 3` | Present, writable. |
| 4 | 512 entries `0x00000083 + n * 0x200000` in `pd` | Identity-map the first 1 GiB with 2 MiB pages. |
| 5 | `movl $pml4, %eax`; `movl %eax, %cr3` | Load the page-table root. |
| 6 | `CR4.PAE`, `EFER.LME` (`MSR 0xC0000080`), `CR0.PG` | Enable long mode. |
| 7 | `lgdt gdt64_desc`; `ljmp $0x08, $long_mode` | Reload `CS` as a 64-bit code segment. |
| 8 | Reload `ds/es/ss/fs/gs` with `0x10`; `leaq stack_top(%rip), %rsp` | 64-bit data segments and stack. |
| 9 | `movl mb_magic(%rip), %edi`; `movl mb_info(%rip), %esi`; `call __init` | SysV register arguments. |
| 10 | `sti`; `cli; hlt; jmp 1b` | Halt if `__init` returns. |

`gdt64` contains null (`0x00`), 64-bit code (`0x08`, access `0x9A`, flags
`0xAF` with `L=1`) and 64-bit data (`0x10`, `0x92`, `0xAF`). Rust's
`arch/x86-64/gdt.rs` installs an equivalent table later in `arch::init()`.

### AP trampoline and `ap_start64`

`ap_startup.rs` copies the trampoline to physical `0x8000` and patches
`ap_tr_gdt_desc.base`, `ap_tr_cr3`, `ap_tr_stack` and `ap_tr_entry64`. It runs:

1. `.code16`: clear segments, `lgdtl`, set `CR0.PE`, far-jump to 32-bit code.
2. `.code32`: load `CR3`, set `CR4.PAE` and `EFER.LME`, set `CR0.PG`, then
   `ljmp $0x18, $ap_tr_long64` (the trampoline GDT's 64-bit code selector).
3. `.code64`: load data selectors `0x20`, take `RSP` from `ap_tr_stack`, and
   jump to `ap_tr_entry64`.

`ap_start64` reloads the data segments, loads `RSP` from the `ap_stack_top`
symbol and calls `ap_entry_rust()`. Note that `ap_stack_top` and the patched
`ap_tr_stack` are separate slots.

### `global_asm!` process stubs

For `target_arch = "x86_64"`, `process/init.rs` exports:

- `fork_save` — pushes `RBX`, `RBP`, `R12`–`R15`, stores `RSP` into the parent
  context (`[RDI]`), rearranges the six arguments for `fork_copy`, restores the
  callee-saved registers and returns.
- `context_switch` — pushes the same six registers, writes `RSP` into `*old`,
  loads `RSP` from `*new`, pops and `ret`s on the new stack.

### Calling convention

x86-64 System V: arguments in `RDI`, `RSI`, `RDX`, `RCX`, `R8`, `R9`, result in
`RAX`, and `RSP` must be 16-byte aligned at the call site.

| Argument | Meaning for `write` | Register |
| --- | --- | --- |
| `n` | `SYS_WRITE = 1` | `edi` |
| `a1` | fd (`1` stdout, `2` stderr) | `esi` |
| `a2` | buffer pointer | `rdx` |
| `a3` | byte count | `rcx` |
| `a4`, `a5` | unused | `r8`, `r9` |
| return | bytes written or `-1` | `rax` |

## EXAMPLES

### Calling `__init` — AT&T ASM (from `boot.S`)

```asm
    leaq stack_top(%rip), %rsp
    movq %rsp, %rbp
    movl mb_magic(%rip), %edi     # a0
    movl mb_info(%rip),  %esi     # a1
    call __init                   # never returns in practice
```

### Calling `write` through a C wrapper — AT&T ASM

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
    .ascii "hello from jawhar\n"
    .set msg_len, . - msg

    .text
    subq $8, %rsp                 # after a call RSP is 8 mod 16; align it
    leaq msg(%rip), %rdx          # a2 = buffer
    movq $msg_len, %rcx           # a3 = length
    movl $1, %esi                 # a1 = STDOUT_FILENO
    movl $1, %edi                 # n  = SYS_WRITE
    xorl %r8d, %r8d               # a4
    xorl %r9d, %r9d               # a5
    call jawhar_syscall
    addq $8, %rsp
    # rax = bytes written, or -1
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
    leaq old_ctx(%rip), %rdi
    leaq new_ctx(%rip), %rsi
    call context_switch           # returns on the new stack
```

### Loading a page-table root — AT&T ASM

```asm
    movq %rax, %cr3               # rax = physical PML4 root (page aligned)
    movq $0x40000000, %rdx
    invlpg (%rdx)                 # invalidate one mapped address
```

## FILES

- `arch/x86-64/boot.S`, `arch/x86-64/linker.ld`.
- `arch/x86-64/gdt.rs`, `arch/x86-64/ap_startup.rs`, `arch/x86-64/smp.rs`.
- `process/init.rs` — x86-64 `global_asm!` block.

## SEE ALSO

[x86](../x86/), [AArch64](../aarch64/), [boot](../../boot.md),
[gdt](../../gdt.md), [smp](../../smp.md), [syscall](../../syscall.md)
