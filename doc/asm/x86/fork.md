# fork(2) — x86 assembly

## NAME

fork - invoke the fork system call from 32-bit x86 assembly

## SYNOPSIS

`jawhar_syscall(SYS_FORK, ...)` — call number `6`; `__syscall!` is variadic (0 to 6 arguments).

## DESCRIPTION

Creates a child by copying the parent's stacks into another process
slot. Returns the child PID to the parent, `0` to the child, or `-1` on failure.
No new address space or privilege level is created.

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
let pid = crate::fork!();
if pid == 0 {
    crate::write!(1, b"child\n", 6);
    crate::exit!(0);
} else if pid > 0 {
    let mut status = 0;
    crate::wait!(pid, &mut status);
}
```

### C

```c
#include <stdint.h>

extern intptr_t jawhar_syscall(uint32_t n, uintptr_t a1, uintptr_t a2,
                               uintptr_t a3, uintptr_t a4, uintptr_t a5);
#define SYS_FORK 6

intptr_t pid = jawhar_syscall(SYS_FORK, 0, 0, 0, 0, 0);
if (pid == 0) {
    /* child */
} else if (pid > 0) {
    /* parent: pid is the child PID */
}
```

### Assembly (AT&T/GAS, cdecl)

```asm
    .text
    pushl $0               # a5
    pushl $0               # a4
    pushl $0               # a3
    pushl $0               # a2
    pushl $0               # a1
    pushl $6               # n  = SYS_FORK
    call jawhar_syscall
    addl $24, %esp
    testl %eax, %eax
    jz   child_path         # eax == 0
    jg   parent_path        # eax = child pid
```

## FILES

- `arch/x86/boot.S`, `arch/x86/linker.ld`.
- `kernel/mod.rs`, `kernel/fork.rs`.
- `process/init.rs` — cdecl `global_asm!` stubs.

## SEE ALSO

[x86 overview](README.md), [System calls](../../syscall.md),
[errno](../../errno.md), [process](../../process.md)
