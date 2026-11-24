# x86(9) — 32-bit x86 assembly

## NAME

x86 - Multiboot entry, GDT, AP trampoline and cdecl process-switch assembly for i386

## SYNOPSIS

`arch/x86/boot.S`, `arch/x86/linker.ld`;
`_start`, `reload_cs`, `ap_start32`, `ap_trampoline_start`, `gdt`, `gdt_desc`;
`fork_save`, `context_switch` in `process/init.rs`

## DESCRIPTION

The 32-bit port uses AT&T/GAS syntax: operands are `source, destination`,
registers are prefixed with `%`, immediates with `$`, and data is emitted with
`.long`, `.word`, `.byte` and `.skip`. `boot.S` is assembled with
`clang --target=i386-unknown-none-elf -c` and linked with `ld.lld -m elf_i386`
using `arch/x86/linker.ld`.

### Multiboot header and `_start`

`linker.ld` places the image at `1M` and keeps the `.multiboot` section first.
`boot.S` emits `MB_MAGIC = 0x1BADB002`, `MB_FLAGS = 3` (module alignment plus
memory information) and the negative checksum. GRUB enters `_start` in 32-bit
protected mode with `EAX` = magic and `EBX` = Multiboot info pointer.

| Step | Instructions | Result |
| --- | --- | --- |
| 1 | `cli`, `movl %eax, mb_magic`, `movl %ebx, mb_info` | Interrupts off; boot values saved in `.data`. |
| 2 | `lgdt gdt_desc` | Kernel GDT loaded. |
| 3 | `movw $0x10, %ax`; reload `ds/es/fs/gs/ss` | Data selectors point at kernel data. |
| 4 | `ljmp $0x08, $reload_cs` | `CS` reloaded with kernel code selector. |
| 5 | `movl $__bss_start, %edi`; `movl $__bss_end, %ecx`; `rep stosb` | `.bss` zeroed before the stack is used. |
| 6 | `movl $stack_top, %esp`; `andl $-16, %esp`; `movl %esp, %ebp` | 16 KiB stack, 16-byte aligned. |
| 7 | `pushl mb_info`; `pushl mb_magic`; `call __init` | `__init(magic, info)` (cdecl). |
| 8 | `addl $16, %esp`; `sti`; `cli; hlt; jmp 1b` | Cleanup, then halt if `__init` returns. |

### GDT

`gdt` has three flat descriptors: null `0x00`, ring-0 code `0x08`
(access `0x9A`, flags `0xCF`) and ring-0 data `0x10` (`0x92`, `0xCF`). The
`0xCF` nibble combines 4 KiB granularity, 32-bit default operand size and the
high limit bits; with limit `0xFFFFF` each segment covers 4 GiB. `gdt.rs`
reproduces the same table with `Desc::new(0, 0xFFFFF, access, 0xC)`.

### AP trampoline

`ap_startup.rs` copies `ap_trampoline_start..ap_trampoline_end` to physical
`0x8000` and patches `ap_tr_gdt_desc.base`, `ap_tr_cr3`, `ap_tr_stack` and
`ap_tr_entry32`. The blob runs in `.code16` first, loads its own GDT, sets
`CR0.PE` and far-jumps to `.code32`. In 32-bit code it optionally loads `CR3`
and `CR0.PG`, then jumps to the patched `ap_tr_entry32`. `ap_start32` reloads
data segments, sets `ESP` from `ap_stack_top`, calls `ap_entry_rust()` and
parks in `cli; hlt`.

### `global_asm!` process stubs

`process/init.rs` declares `fork_save` and `context_switch` as global assembly
for `target_arch = "x86"`. Both push `EBX`, `EBP`, `ESI`, `EDI`. `fork_save`
stores the current `ESP` into the parent context pointer, shuffles the six
cdecl arguments and calls `fork_copy`, then restores the registers.
`context_switch` writes `ESP` into `*old`, loads `ESP` from `*new`, pops the
four registers and `ret`s on the new stack.

### Calling convention

cdecl passes arguments right-to-left on the stack and returns in `EAX`; the
caller removes them. A six-argument C-ABI call therefore looks like this:

```
ESP+24  a5        (top after all pushes)
ESP+20  a4
ESP+16  a3
ESP+12  a2
ESP+8   a1
ESP+4   a0
ESP+0   return address (after `call`)
```

## EXAMPLES

### Calling `__init` — AT&T ASM (from `boot.S`)

```asm
    subl $8, %esp
    pushl mb_info          # a1
    pushl mb_magic         # a0
    call __init            # never returns in practice
    addl $16, %esp
```

### Calling `write` through a C wrapper — AT&T ASM

`write!` expands to a direct Rust call, so export a C-ABI wrapper first:

```rust
#[no_mangle]
pub extern "C" fn jawhar_syscall(
    n: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize,
) -> isize {
    crate::__syscall!(n, a1, a2, a3, a4, a5)
}
```

```asm
    .section .rodata
msg:
    .ascii "hello\n"
    .set msg_len, . - msg

    .text
    pushl $0               # a5
    pushl $0               # a4
    pushl $msg_len         # a3 = length
    pushl $msg             # a2 = buffer
    pushl $1               # a1 = stdout
    pushl $1               # a0 = SYS_WRITE
    call jawhar_syscall
    addl $24, %esp
    # eax = bytes written, or -1
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
    pushl $new_ctx
    pushl $old_ctx
    call context_switch    # returns on the new stack
    addl $8, %esp
```

## FILES

- `arch/x86/boot.S`, `arch/x86/linker.ld`.
- `arch/x86/gdt.rs`, `arch/x86/ap_startup.rs`, `arch/x86/smp.rs`.
- `process/init.rs` — x86 `global_asm!` block.

## SEE ALSO

[x86-64](../x86_64/), [AArch64](../aarch64/), [boot](../../boot.md),
[gdt](../../gdt.md), [smp](../../smp.md), [syscall](../../syscall.md)
