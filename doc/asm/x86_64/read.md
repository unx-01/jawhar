# read(2) — x86_64 assembly

## NAME

read - invoke the read system call from x86-64 assembly

## SYNOPSIS

`jawhar_syscall(SYS_READ, ...)` — call number `2`; `__syscall!` is variadic (0 to 6 arguments).

## DESCRIPTION

Intended to read `count` bytes from `fd` into `buf`, but the kernel
implementation is a stub that always returns `-1`. No `errno` value is set.

The Rust convenience call is listed first. C and assembly reach the same
dispatch through an exported wrapper, because the Rust symbol is mangled:

```rust
#[no_mangle]
pub extern "C" fn jawhar_syscall(
    n: u32, a1: usize, a2: usize, a3: usize, a4: usize, a5: usize,
) -> isize {
    crate::__syscall!(n, a1, a2, a3, a4, a5)
}
```

## EXAMPLES

### Rust

```rust
let mut buf = [0u8; 16];
let n = crate::read!(0, &mut buf[..], buf.len());   // stub: -1
```

### C

```c
#include <stdint.h>

extern intptr_t jawhar_syscall(uint32_t n, uintptr_t a1, uintptr_t a2,
                               uintptr_t a3, uintptr_t a4, uintptr_t a5);
#define SYS_READ 2

char buf[16];
intptr_t n = jawhar_syscall(SYS_READ, 0, (uintptr_t)buf,
                            sizeof(buf), 0, 0);   // -1 today
```

### Assembly (AT&T/GAS, System V)

```asm
    .section .bss
buf:    .skip 16

    .text
    subq $8, %rsp
    leaq buf(%rip), %rdx   # a2 = buffer
    movq $16, %rcx         # a3 = count
    movl $0, %esi          # a1 = fd
    movl $2, %edi          # n  = SYS_READ
    xorl %r8d, %r8d
    xorl %r9d, %r9d
    call jawhar_syscall
    addq $8, %rsp
    # rax = -1 until read is implemented
```

## FILES

- `arch/x86-64/boot.S`, `arch/x86-64/linker.ld`.
- `kernel/mod.rs`, `kernel/read.rs`.
- `process/init.rs` — SysV `global_asm!` stubs.

## SEE ALSO

[x86_64 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
