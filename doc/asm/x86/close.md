# close(2) — x86 assembly

## NAME

close - invoke the close system call from 32-bit x86 assembly

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

### Assembly (AT&T/GAS, cdecl)

```asm
    .text
    pushl $0               # a5
    pushl $0               # a4
    pushl $0               # a3
    pushl $0               # a2
    pushl $1               # a1 = fd
    pushl $4               # n  = SYS_CLOSE
    call jawhar_syscall
    addl $24, %esp
    # eax = -1
```

## FILES

- `arch/x86/boot.S`, `arch/x86/linker.ld`.
- `kernel/mod.rs`, `kernel/close.rs`.
- `process/init.rs` — cdecl `global_asm!` stubs.

## SEE ALSO

[x86 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
