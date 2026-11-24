# close(2) — x86_64 assembly

## NAME

close - invoke the close system call from x86-64 assembly

## SYNOPSIS

`jawhar_syscall(SYS_CLOSE, ...)` — call number `4`; `__syscall!` is variadic (0 to 6 arguments).

## DESCRIPTION

Placeholder for closing a descriptor. With no fd table it always
returns `-1` and does not set `errno`. There is no dedicated `close!` macro.

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
let ret = crate::__syscall!(
    crate::syscall::SYS_CLOSE, 1, 0, 0, 0, 0,
);   // -1
```

### C

```c
#include <stdint.h>

extern intptr_t jawhar_syscall(uint32_t n, uintptr_t a1, uintptr_t a2,
                               uintptr_t a3, uintptr_t a4, uintptr_t a5);
#define SYS_CLOSE 4

intptr_t ret = jawhar_syscall(SYS_CLOSE, 1, 0, 0, 0, 0);
```

### Assembly (AT&T/GAS, System V)

```asm
    .text
    subq $8, %rsp
    movl $1, %esi          # a1 = fd
    xorl %edx, %edx
    xorl %ecx, %ecx
    xorl %r8d, %r8d
    xorl %r9d, %r9d
    movl $4, %edi          # n  = SYS_CLOSE
    call jawhar_syscall
    addq $8, %rsp
    # rax = -1
```

## FILES

- `arch/x86-64/boot.S`, `arch/x86-64/linker.ld`.
- `kernel/mod.rs`, `kernel/close.rs`.
- `process/init.rs` — SysV `global_asm!` stubs.

## SEE ALSO

[x86_64 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
