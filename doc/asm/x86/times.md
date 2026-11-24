# times(2) — x86 assembly

## NAME

times - invoke the times system call from 32-bit x86 assembly

## SYNOPSIS

`jawhar_syscall(SYS_TIMES, ...)` — call number `10`; `__syscall!` is variadic (0 to 6 arguments).

## DESCRIPTION

Fills a `Tms` structure with user, system, child-user and child-system
tick counts and returns the current tick count. A null buffer sets `EFAULT`
and returns `-1`.

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
let mut t = crate::time::Tms::default();
let ticks = crate::times!(&mut t);
```

### C

```c
#include <stdint.h>

extern intptr_t jawhar_syscall(uint32_t n, uintptr_t a1, uintptr_t a2,
                               uintptr_t a3, uintptr_t a4, uintptr_t a5);
#define SYS_TIMES 10

typedef long clock_t;
typedef struct {
    clock_t tms_utime, tms_stime, tms_cutime, tms_cstime;
} Tms;

Tms t = {0};
intptr_t ticks = jawhar_syscall(SYS_TIMES, (uintptr_t)&t, 0, 0, 0, 0);
```

### Assembly (AT&T/GAS, cdecl)

```asm
    .section .bss
tms_buf: .skip 32          # four 32-bit clock_t values

    .text
    pushl $0               # a5
    pushl $0               # a4
    pushl $0               # a3
    pushl $0               # a2
    pushl $tms_buf         # a1 = &Tms
    pushl $10              # n  = SYS_TIMES
    call jawhar_syscall
    addl $24, %esp
    # eax = tick count
```

## FILES

- `arch/x86/boot.S`, `arch/x86/linker.ld`.
- `kernel/mod.rs`, `kernel/times.rs`.
- `process/init.rs` — cdecl `global_asm!` stubs.

## SEE ALSO

[x86 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
