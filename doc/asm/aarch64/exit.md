# exit(2) — aarch64 assembly

## NAME

exit - invoke the exit system call from AArch64 assembly

## SYNOPSIS

`jawhar_syscall(SYS_EXIT, ...)` — call number `7`; `__syscall!` is variadic (0 to 6 arguments).

## DESCRIPTION

Marks the current process as a zombie and switches to the next
runnable process. The status is narrowed to `u16`; the call never returns.

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
crate::exit!(0);   // never returns
```

### C

```c
#include <stdint.h>

extern intptr_t jawhar_syscall(uint32_t n, uintptr_t a1, uintptr_t a2,
                               uintptr_t a3, uintptr_t a4, uintptr_t a5);
#define SYS_EXIT 7

jawhar_syscall(SYS_EXIT, 0, 0, 0, 0, 0);   /* never returns */
```

### Assembly (GNU as, AAPCS64)

```asm
    .text
    mov x1, #0             // a1 = status
    mov x2, #0
    mov x3, #0
    mov x4, xzr
    mov x5, xzr
    mov x0, #7             // n  = SYS_EXIT
    bl  jawhar_syscall
    // no return; the context switch never comes back
```

## FILES

- `arch/aarch64/boot.S`, `arch/aarch64/linker.ld`.
- `kernel/mod.rs`, `kernel/exit.rs`.
- `process/init.rs` — AAPCS64 `global_asm!` stubs.

## SEE ALSO

[aarch64 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
