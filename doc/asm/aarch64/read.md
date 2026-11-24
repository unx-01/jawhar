# read(2) — aarch64 assembly

## NAME

read - invoke the read system call from AArch64 assembly

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

### Assembly (GNU as, AAPCS64)

```asm
    .section .bss
buf:    .skip 16

    .text
    adrp x2, buf
    add  x2, x2, :lo12:buf  // a2 = buffer
    mov  x3, #16            // a3 = count
    mov  x1, #0             // a1 = fd
    mov  x0, #2             // n  = SYS_READ
    mov  x4, xzr
    mov  x5, xzr
    bl   jawhar_syscall
    // x0 = -1 until read is implemented
```

## FILES

- `arch/aarch64/boot.S`, `arch/aarch64/linker.ld`.
- `kernel/mod.rs`, `kernel/read.rs`.
- `process/init.rs` — AAPCS64 `global_asm!` stubs.

## SEE ALSO

[aarch64 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
