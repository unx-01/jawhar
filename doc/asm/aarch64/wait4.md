# wait4(2) — aarch64 assembly

## NAME

wait4 - invoke the wait4 system call from AArch64 assembly

## SYNOPSIS

`jawhar_syscall(SYS_WAIT4, ...)` — call number `9`; `__syscall!` is variadic (0 to 6 arguments).

## DESCRIPTION

Waits for a child process and stores its status through a pointer.
Returns the reaped PID or `-1` when no matching child exists. Options and
`rusage` are not accepted.

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
let mut status = 0i32;
let reaped = crate::wait!(-1, &mut status);
```

### C

```c
#include <stdint.h>

extern intptr_t jawhar_syscall(uint32_t n, uintptr_t a1, uintptr_t a2,
                               uintptr_t a3, uintptr_t a4, uintptr_t a5);
#define SYS_WAIT4 9

int status = 0;
intptr_t reaped = jawhar_syscall(SYS_WAIT4, (uintptr_t)-1,
                                 (uintptr_t)&status, 0, 0, 0);
```

### Assembly (GNU as, AAPCS64)

```asm
    .section .bss
status: .word 0
        .align 3

    .text
    adrp x2, status
    add  x2, x2, :lo12:status // a2 = &status
    movn x1, #0              // a1 = -1 (any child)
    mov  x3, #0
    mov  x4, xzr
    mov  x5, xzr
    mov  x0, #9              // n  = SYS_WAIT4
    bl   jawhar_syscall
    // x0 = reaped pid, or -1
```

## FILES

- `arch/aarch64/boot.S`, `arch/aarch64/linker.ld`.
- `kernel/mod.rs`, `kernel/wait4.rs`.
- `process/init.rs` — AAPCS64 `global_asm!` stubs.

## SEE ALSO

[aarch64 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
