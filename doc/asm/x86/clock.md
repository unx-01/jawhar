# clock(2) — x86 assembly

## NAME

clock - invoke the clock system call from 32-bit x86 assembly

## SYNOPSIS

`jawhar_syscall(SYS_CLOCK, ...)` — call number `11`; `__syscall!` is variadic (0 to 6 arguments).

## DESCRIPTION

Returns the current process's user plus system CPU time in ticks
(`utime + stime`). It takes no arguments and uses no buffer.

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
let cpu_ticks = crate::clock!();
```

### C

```c
#include <stdint.h>

extern intptr_t jawhar_syscall(uint32_t n, uintptr_t a1, uintptr_t a2,
                               uintptr_t a3, uintptr_t a4, uintptr_t a5);
#define SYS_CLOCK 11

intptr_t cpu_ticks = jawhar_syscall(SYS_CLOCK, 0, 0, 0, 0, 0);
```

### Assembly (AT&T/GAS, cdecl)

```asm
    .text
    pushl $0               # a5
    pushl $0               # a4
    pushl $0               # a3
    pushl $0               # a2
    pushl $0               # a1
    pushl $11              # n  = SYS_CLOCK
    call jawhar_syscall
    addl $24, %esp
    # eax = utime + stime in ticks
```

## FILES

- `arch/x86/boot.S`, `arch/x86/linker.ld`.
- `kernel/mod.rs`, `kernel/clock.rs`.
- `process/init.rs` — cdecl `global_asm!` stubs.

## SEE ALSO

[x86 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
